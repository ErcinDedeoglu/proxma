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

var logger *Logger

func init() {
	// Initialize logger with debug mode based on environment variable
	debug := getEnvOrDefault("PROXMA_DEBUG", "false") == "true"
	logger = NewLogger(debug)
}

func handleContainer(cli *client.Client, c container.Summary) error {
	labels := c.Labels

	if !hasProxmaLabels(labels) {
		logger.Debug("Skipping container %s: no proxma labels", c.Names[0])
		return nil
	}

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

	// Print formatted container configuration using logger
	logger.Info("\n" + logger.FormatContainerConfig(c, ip, sslConfig, mainHostsSlice, redirects, port))

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

	logger.Success("Generated configuration for %s", c.Names[0])
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

func ensureDefaultConfig(configDir string) error {
	defaultConfPath := filepath.Join(configDir, "default.conf")

	// Check if default config already exists
	if _, err := os.Stat(defaultConfPath); err == nil {
		return nil
	}

	// Create default configuration content
	defaultConfig := `server {
    listen 80 default_server;
    listen [::]:80 default_server;
    
    server_name _;
    root /var/www/default_page;
    
    location / {
        try_files $uri $uri/ =404;
    }

    location /.well-known/acme-challenge/ {
        root /var/www/certbot;
    }
}`

	// Write default configuration
	err := os.WriteFile(defaultConfPath, []byte(defaultConfig), 0644)
	if err != nil {
		return fmt.Errorf("failed to write default configuration: %v", err)
	}

	logger.Info("Created default nginx configuration")
	return nil
}

func updateConfigFiles(tempDir, finalDir string) error {
	// Read all new configuration files
	newConfigs, err := filepath.Glob(filepath.Join(tempDir, "*.conf"))
	if err != nil {
		return fmt.Errorf("error reading new configs: %v", err)
	}

	// Read existing configuration files
	existingConfigs, err := filepath.Glob(filepath.Join(finalDir, "*.conf"))
	if err != nil {
		return fmt.Errorf("error reading existing configs: %v", err)
	}

	// Create a map of existing configs for quick lookup
	existingConfigMap := make(map[string]bool)
	for _, conf := range existingConfigs {
		existingConfigMap[filepath.Base(conf)] = true
	}

	// Remove old configurations that are no longer needed
	for _, oldConf := range existingConfigs {
		baseName := filepath.Base(oldConf)
		newConfPath := filepath.Join(tempDir, baseName)
		if _, err := os.Stat(newConfPath); os.IsNotExist(err) {
			if err := os.Remove(oldConf); err != nil {
				logger.Warning("Failed to remove old config %s: %v", oldConf, err)
			} else {
				logger.Debug("Removed old config: %s", baseName)
			}
		}
	}

	// Copy new configurations
	for _, newConf := range newConfigs {
		baseName := filepath.Base(newConf)
		targetPath := filepath.Join(finalDir, baseName)

		// Read new config content
		content, err := os.ReadFile(newConf)
		if err != nil {
			return fmt.Errorf("error reading new config %s: %v", baseName, err)
		}

		// Write to target location
		if err := os.WriteFile(targetPath, content, 0644); err != nil {
			return fmt.Errorf("error writing config %s: %v", baseName, err)
		}

		logger.Debug("Updated config: %s", baseName)
	}

	return nil
}

func generateConfigs(cli *client.Client) {
	// Acquire lock to ensure only one instance runs simultaneously
	lockFilePath := "/tmp/proxma.lock"
	lockFile, err := os.OpenFile(lockFilePath, os.O_CREATE|os.O_RDWR, 0644)
	if err != nil {
		logger.Error("Cannot create/open lock file: %v", err)
		return
	}
	defer lockFile.Close()

	err = syscall.Flock(int(lockFile.Fd()), syscall.LOCK_EX|syscall.LOCK_NB)
	if err != nil {
		logger.Warning("Another configuration regeneration is running. Skipping this regeneration.")
		return
	}
	defer syscall.Flock(int(lockFile.Fd()), syscall.LOCK_UN)

	// Print start process message
	logger.Info("\n%s", logger.StartProcess())

	// List all running containers
	containers, err := listContainers(cli)
	if err != nil {
		logger.Error("Failed to list containers: %v", err)
		return
	}

	// Directory setup
	tempDir := "/tmp/nginx_conf_temp"
	finalDir := "/etc/nginx/conf.d"

	// Clean and create temp directory
	if err := os.RemoveAll(tempDir); err != nil {
		logger.Debug("Removing temp directory: %v", err)
	}
	if err := os.MkdirAll(tempDir, 0755); err != nil {
		logger.Error("Failed to create temp directory: %v", err)
		return
	}

	// Process containers and generate configs
	problematicContainers := 0
	configuredContainers := 0
	var errors []string

	// Process each container
	for _, c := range containers {
		err := handleContainer(cli, c)
		if err != nil {
			problematicContainers++
			errors = append(errors, fmt.Sprintf("%s: %v", strings.TrimPrefix(c.Names[0], "/"), err))
			logger.Error("Container %s: %v", c.Names[0], err)
		} else if hasProxmaLabels(c.Labels) {
			configuredContainers++
		}
	}

	// Print summary report
	logger.Info("\n%s", logger.CreateSummaryReport(len(containers), configuredContainers, problematicContainers, errors))

	// If no valid configurations were generated, clean up and exit
	newConfigs, err := filepath.Glob(filepath.Join(tempDir, "*.conf"))
	if err != nil {
		logger.Error("Failed to check for generated configs: %v", err)
		return
	}

	if len(newConfigs) == 0 {
		logger.Warning("No valid configurations were generated")
		// Ensure default configuration exists
		err = ensureDefaultConfig(finalDir)
		if err != nil {
			logger.Error("Failed to ensure default configuration: %v", err)
		}
		return
	}

	// Validate nginx configs
	logger.Info("Testing nginx configuration...")
	output, err := exec.Command("nginx", "-t", "-c", "/etc/nginx/nginx.conf").CombinedOutput()
	if err != nil {
		logger.Error("Nginx configuration test failed: %s", output)
		return
	}
	logger.Success("Nginx configuration test passed")

	// Update configuration files
	logger.Info("Updating nginx configuration files...")
	if err := updateConfigFiles(tempDir, finalDir); err != nil {
		logger.Error("Failed to update configuration files: %v", err)
		return
	}
	logger.Success("Configuration files updated successfully")

	// Reload NGINX gracefully
	logger.Info("Reloading nginx...")
	if err := exec.Command("nginx", "-s", "reload").Run(); err != nil {
		logger.Error("Nginx reload failed: %v", err)
		// Try to recover by ensuring default config
		if err := ensureDefaultConfig(finalDir); err != nil {
			logger.Error("Failed to recover with default configuration: %v", err)
		}
		return
	}
	logger.Success("Nginx configuration reloaded successfully")

	// Check and handle SSL certificates if enabled
	globalSSLEnabled, _ := strconv.ParseBool(getEnvOrDefault("PROXMA_SSL", "false"))
	if globalSSLEnabled {
		logger.Info("Processing SSL certificates...")
		issueSSLCertsIfMissing()
	} else {
		logger.Info("SSL is globally disabled, skipping SSL certificate checks")
	}

	// Final cleanup
	if err := os.RemoveAll(tempDir); err != nil {
		logger.Debug("Failed to clean up temp directory: %v", err)
	}

	logger.Success("Configuration generation completed successfully")
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
