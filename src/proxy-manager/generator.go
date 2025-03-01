package main

import (
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"syscall"

	"github.com/docker/docker/api/types/container"
	"github.com/docker/docker/client"
)

func handleContainer(cli *client.Client, c container.Summary) error {
	labels := c.Labels

	// Check if this container has any proxma-related labels
	if !hasProxmaLabels(labels) {
		return nil // Silently skip containers without proxma labels
	}

	// Now check required labels
	hostsLabel, hasHosts := labels["proxma.hosts"]
	port, hasPort := labels["proxma.port"]
	if !(hasHosts && hasPort) {
		return fmt.Errorf("container %s has proxma labels but missing required labels (hosts: %v, port: %v)",
			c.Names[0], hasHosts, hasPort)
	}

	sslConfig := getSSLConfig(labels)
	redirectsLabel, hasRedirects := labels["proxma.redirects"]

	containerDetails, err := inspectContainer(cli, c.ID)
	if err != nil {
		return fmt.Errorf("failed inspecting container %s: %w", c.Names[0], err)
	}

	ip := ""
	for _, net := range containerDetails.NetworkSettings.Networks {
		ip = net.IPAddress
		break
	}
	if ip == "" {
		return fmt.Errorf("no valid IP found for container %s", c.Names[0])
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
				}
			}
		}
	}

	if len(mainHostsSet) == 0 {
		return fmt.Errorf("no valid hosts remaining after redirect processing for container %s", c.Names[0])
	}

	mainHostsSlice := make([]string, 0, len(mainHostsSet))
	for host := range mainHostsSet {
		mainHostsSlice = append(mainHostsSlice, host)
	}

	confDir := "/tmp/nginx_conf_temp"
	confName := fmt.Sprintf("%s/%s.conf", confDir, strings.TrimPrefix(c.Names[0], "/"))
	confFile, err := os.Create(confName)
	if err != nil {
		return fmt.Errorf("error creating nginx config file for %s: %w", c.Names[0], err)
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
		return fmt.Errorf("error executing nginx template for %s: %w", c.Names[0], err)
	}

	return nil
}

func hasProxmaLabels(labels map[string]string) bool {
	for key := range labels {
		if strings.HasPrefix(key, "proxma.") {
			return true
		}
	}
	return false
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
	configuredContainers := 0

	for _, c := range containers {
		err := handleContainer(cli, c)
		if err != nil {
			problematicContainers++
			log.Printf("⚠️ %v", err)
		} else if hasProxmaLabels(c.Labels) {
			configuredContainers++
		}
	}

	if problematicContainers > 0 {
		log.Printf("Completed regeneration with ⚠️ %d problematic containers out of %d configured containers.",
			problematicContainers, configuredContainers)
	} else if configuredContainers > 0 {
		log.Printf("✅ Successfully configured %d containers.", configuredContainers)
	} else {
		log.Printf("ℹ️ No containers configured for proxying.")
	}

	// Validate configs
	output, err := exec.Command("nginx", "-t", "-c", "/etc/nginx/nginx.conf").CombinedOutput()
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

	// Check SSL certificates if needed
	globalSSLEnabled, _ := strconv.ParseBool(getEnvOrDefault("PROXMA_SSL", "false"))
	if globalSSLEnabled {
		issueSSLCertsIfMissing()
	} else {
		log.Println("SSL is globally disabled, skipping SSL certificate checks.")
	}
}

func issueSSLCertsIfMissing() {
	globalSSLEnabled, _ := strconv.ParseBool(getEnvOrDefault("PROXMA_SSL", "false"))
	if !globalSSLEnabled {
		log.Println("SSL is globally disabled, skipping SSL certificate checks.")
		return
	}

	confFiles, err := filepath.Glob("/etc/nginx/conf.d/*.conf")
	if err != nil {
		log.Println("Error finding conf files:", err)
		return
	}

	email := getEnvOrDefault("PROXMA_SSL_EMAIL", "")
	if email == "" {
		log.Println("🚨 PROXMA_SSL_EMAIL not set but SSL is enabled. SSL certificate issuance will be skipped.")
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
