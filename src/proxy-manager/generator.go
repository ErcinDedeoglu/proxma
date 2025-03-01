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
		log.Println("PROXMA_SSL_EMAIL environment variable is not set, skipping certificate issuance.")
		return
	}

	for _, confFile := range confFiles {
		content, err := os.ReadFile(confFile)
		if err != nil {
			log.Printf("Could not read conf file %s: %v", confFile, err)
			continue
		}

		if strings.Contains(string(content), "/.well-known/acme-challenge/") {

			// Handle multiple domains properly
			domains := extractDomainsFromConf(string(content))
			if len(domains) == 0 {
				log.Printf("No valid domains found in conf file: %s", confFile)
				continue
			}

			primaryDomain := domains[0]
			certPath := "/etc/certificates/live/" + primaryDomain + "/fullchain.pem"

			if _, err := os.Stat(certPath); err == nil {
				log.Printf("SSL certificate already exists for domain %s, skipping cert issuance.", primaryDomain)
				continue // Skip cert issuance if cert already present
			}

			// SSL cert missing, issue a new one
			log.Printf("Issuing SSL Certificate for domains: %v", domains)

			certbotArgs := []string{
				"certonly", "--webroot",
				"-w", "/var/www/certbot",
				"--agree-tos", "--non-interactive",
				"--config-dir", "/etc/certificates",
				"--email", email,
			}

			for _, domain := range domains {
				certbotArgs = append(certbotArgs, "-d", domain)
			}

			cmd := exec.Command("certbot", certbotArgs...)
			out, err := cmd.CombinedOutput()
			if err != nil {
				log.Printf("Certbot issuance failed for %v: %s (%v)", domains, string(out), err)
				continue
			} else {
				log.Printf("Certbot successfully issued for domains: %v. Output:\n%s", domains, string(out))

				// Reload NGINX clearly after successful certbot run
				err = exec.Command("nginx", "-s", "reload").Run()
				if err != nil {
					log.Printf("Failed to reload NGINX after cert issuance: %v", err)
				} else {
					log.Printf("Successfully reloaded NGINX after certificate issuance for domains: %v", domains)
				}
			}
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
