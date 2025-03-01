package main

import (
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"

	"github.com/docker/docker/api/types/container"
	"github.com/docker/docker/client"
)

func handleContainer(cli *client.Client, c container.Summary) error {
	labels := c.Labels
	sslConfig := getSSLConfig(labels)
	hostsLabel, hasHosts := labels["proxma.hosts"]
	port, hasPort := labels["proxma.port"]
	redirectsLabel, hasRedirects := labels["proxma.redirects"]

	if !(hasHosts && hasPort) {
		return fmt.Errorf("container missing required labels (hosts: %v, port: %v)", hasHosts, hasPort)
	}

	containerDetails, err := inspectContainer(cli, c.ID)
	if err != nil {
		return fmt.Errorf("failed inspecting container: %w", err)
	}

	ip := ""
	for _, net := range containerDetails.NetworkSettings.Networks {
		ip = net.IPAddress
		break
	}
	if ip == "" {
		return fmt.Errorf("no valid IP found for container")
	}

	mainHostsSet := make(map[string]bool)
	for _, h := range strings.Split(hostsLabel, ",") {
		cleanedHost := strings.TrimSpace(h)
		if cleanedHost == "" {
			continue
		}
		mainHostsSet[cleanedHost] = true
	}

	redirects := []Redirect{}
	if hasRedirects {
		redirectPairs := strings.Split(redirectsLabel, ",")
		for _, pair := range redirectPairs {
			parts := strings.Split(pair, ">")
			if len(parts) == 2 {
				src := strings.TrimSpace(parts[0])
				dst := strings.TrimSpace(parts[1])
				if src != "" && dst != "" {
					redirects = append(redirects, Redirect{src, dst})
					delete(mainHostsSet, src)
				} else {
					log.Printf("⚠️ Invalid redirect defined in container %s: '%s'", c.Names[0], pair)
				}
			} else {
				log.Printf("⚠️ Malformed redirect format in container %s: '%s'", c.Names[0], pair)
			}
		}
	}

	if len(mainHostsSet) == 0 {
		return fmt.Errorf("no main host left after redirect processing for container")
	}

	mainHostsSlice := make([]string, 0, len(mainHostsSet))
	for host := range mainHostsSet {
		mainHostsSlice = append(mainHostsSlice, host)
	}

	// write NGINX config - assuming temp dir pattern from previous step
	confDir := "/tmp/nginx_conf_temp"
	confName := fmt.Sprintf("%s/%s.conf", confDir, strings.TrimPrefix(c.Names[0], "/"))
	confFile, err := os.Create(confName)
	if err != nil {
		return fmt.Errorf("error creating nginx config file: %w", err)
	}
	defer confFile.Close()

	err = nginxTemplate.Execute(confFile, map[string]interface{}{
		"MainHosts":   strings.Join(mainHostsSlice, " "),
		"IP":          ip,
		"Port":        port,
		"Redirects":   redirects,
		"SSL":         sslConfig.Enabled,
		"SSLProvider": sslConfig.Provider,
		"SSLEmail":    sslConfig.Email,
	})
	if err != nil {
		return fmt.Errorf("error executing nginx template: %w", err)
	}

	return nil
}

func generateConfigs(cli *client.Client) {

	// Acquire lock to ensure only one instance runs simultaneously
	lockFilePath := "/tmp/proxma.lock"
	lockFile, err := os.OpenFile(lockFilePath, os.O_CREATE|os.O_RDWR, 0644)
	if err != nil {
		log.Printf("🚨 Cannot create/open lock file: %v", err)
		return
	}
	defer lockFile.Close()

	err = syscall.Flock(int(lockFile.Fd()), syscall.LOCK_EX|syscall.LOCK_NB)
	if err != nil {
		log.Println("🔒 Another configuration regeneration is running. Skipping this regeneration.")
		return
	}
	defer syscall.Flock(int(lockFile.Fd()), syscall.LOCK_UN)

	log.Println("🔓 Lock acquired. Regenerating configurations now.")

	// Your complete existing logic: atomic swaps, SSL checks, per-container error handling
	containers, err := listContainers(cli)
	if err != nil {
		log.Printf("Error listing containers: %v", err)
		return
	}

	tempDir := "/tmp/nginx_conf_temp"
	finalDir := "/etc/nginx/conf.d"
	backupDir := "/etc/nginx/conf.d.backup"

	os.RemoveAll(tempDir)
	os.MkdirAll(tempDir, 0755)

	problematicContainers := 0
	for _, c := range containers {
		if err := handleContainer(cli, c); err != nil {
			problematicContainers++
			log.Printf("⚠️ Error handling container %s (%s): %v", c.Names[0], c.ID, err)
			continue
		}
	}

	if problematicContainers > 0 {
		log.Printf("Completed regeneration with ⚠️ %d problematic containers.", problematicContainers)
	} else {
		log.Println("All containers processed successfully ✅.")
	}

	// Validate configs
	output, err := exec.Command("nginx", "-t", "-c", "/etc/nginx/nginx.conf", "-g", "include "+tempDir+"/*.conf;").CombinedOutput()
	if err != nil {
		log.Printf("Nginx test failed: %s", output)
		return
	}

	// Atomic directory replacement
	os.RemoveAll(backupDir)
	if _, err := os.Stat(finalDir); err == nil {
		os.Rename(finalDir, backupDir)
	}
	os.Rename(tempDir, finalDir)

	// Reload NGINX gracefully
	err = exec.Command("nginx", "-s", "reload").Run()
	if err != nil {
		log.Printf("Reload failed; reverting to backup: %v", err)
		os.RemoveAll(finalDir)
		os.Rename(backupDir, finalDir)
		exec.Command("nginx", "-s", "reload").Run()
		return
	}

	issueSSLCertsIfMissing()
}

func issueSSLCertsIfMissing() {
	confFiles, err := filepath.Glob("/etc/nginx/conf.d/*.conf")
	if err != nil {
		log.Println("Error finding conf files:", err)
		return
	}

	email := os.Getenv("PROXMA_SSL_EMAIL")
	if email == "" {
		log.Println("🚨 PROXMA_SSL_EMAIL not set, skipping SSL issuance.")
		return
	}

	for _, confFile := range confFiles {
		content, err := os.ReadFile(confFile)
		if err != nil {
			log.Printf("Could not read conf file (%s): %v", confFile, err)
			continue
		}

		if !strings.Contains(string(content), "/.well-known/acme-challenge/") {
			continue
		}

		domains := extractDomainsFromConf(string(content))
		if len(domains) == 0 {
			log.Printf("No domains found in conf file: %s", confFile)
			continue
		}

		primaryDomain := domains[0]
		certPath := "/etc/certificates/live/" + primaryDomain + "/fullchain.pem"

		if _, err := os.Stat(certPath); err == nil {
			log.Printf("SSL certificate already exists for domain %s.", primaryDomain)
			continue
		}

		log.Printf("🌐 Issuing SSL for: %v", domains)

		certbotArgs := []string{"certonly", "--webroot", "-w", "/var/www/certbot",
			"--agree-tos", "--non-interactive",
			"--config-dir", "/etc/certificates",
			"--email", email}
		for _, domain := range domains {
			certbotArgs = append(certbotArgs, "-d", domain)
		}

		cmd := exec.Command("certbot", certbotArgs...)
		out, err := cmd.CombinedOutput()

		if err != nil {
			log.Printf("🚨 Certbot failed for %v: %v\nOUTPUT:%s\nRetry will occur periodically.", domains, err, string(out))

			// Clearly mark failure explicitly to alert operator/admin
			failureMarkerPath := fmt.Sprintf("/tmp/cert_issue_failed_%s.marker", primaryDomain)
			os.WriteFile(failureMarkerPath, []byte(fmt.Sprintf("Failed: %v\n%s", err, string(out))), 0644)

			continue
		}

		log.Printf("✅ Certbot success for: %v.\nOutput:%s", domains, string(out))

		err = exec.Command("nginx", "-s", "reload").Run()
		if err != nil {
			log.Printf("🚨 NGINX reload after Certbot failed: %v", err)
		} else {
			log.Printf("🔄 NGINX reloaded after SSL issuance: %v", domains)

			// Remove any failure marker explicitly if previously existed
			failureMarkerPath := fmt.Sprintf("/tmp/cert_issue_failed_%s.marker", primaryDomain)
			os.Remove(failureMarkerPath)
		}
	}
}

// helper function (robust multiple domains parsing)
func extractDomainsFromConf(conf string) []string {
	lines := strings.Split(conf, "\n")
	var domains []string
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, "server_name") {
			fields := strings.Fields(line)
			if len(fields) >= 2 {
				for _, domain := range fields[1:] {
					domain = strings.Trim(domain, ";")
					domains = append(domains, domain)
				}
			}
		}
	}
	return domains
}
