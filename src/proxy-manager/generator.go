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

// Supporting types
type ContainerInfo struct {
	Name        string
	Domains     []string
	Port        string
	SSL         bool
	SSLEmail    string
	SSLProvider string
	Redirects   []Redirect
}

type ContainerError struct {
	Name  string
	Error error
}

type CertificateInfo struct {
	Domain   string
	Status   string
	Provider string
}

var logger *Logger

func init() {
	debug := getEnvOrDefault("PROXMA_DEBUG", "false") == "true"
	logger = NewLogger(debug)
}

func processContainer(cli *client.Client, c container.Summary) (*ContainerConfig, error) {
	labels := c.Labels

	// Validate required labels
	hostsLabel, hasHosts := labels["proxma.hosts"]
	port, hasPort := labels["proxma.port"]
	if !(hasHosts && hasPort) {
		return nil, fmt.Errorf("missing required labels (hosts: %v, port: %v)", hasHosts, hasPort)
	}

	// Get container IP
	containerDetails, err := inspectContainer(cli, c.ID)
	if err != nil {
		return nil, fmt.Errorf("failed inspecting container: %v", err)
	}

	ip := ""
	for _, net := range containerDetails.NetworkSettings.Networks {
		ip = net.IPAddress
		break
	}
	if ip == "" {
		return nil, fmt.Errorf("no valid IP address found")
	}

	// Process domains
	domains := make([]string, 0)
	domainsSet := make(map[string]bool)
	for _, h := range strings.Split(hostsLabel, ",") {
		domain := strings.TrimSpace(h)
		if domain != "" {
			domainsSet[domain] = true
		}
	}

	// Process redirects
	redirects := []Redirect{}
	if redirectsLabel, hasRedirects := labels["proxma.redirects"]; hasRedirects {
		redirectPairs := strings.Split(redirectsLabel, ",")
		for _, pair := range redirectPairs {
			parts := strings.Split(pair, ">")
			if len(parts) == 2 {
				src := strings.TrimSpace(parts[0])
				dst := strings.TrimSpace(parts[1])
				if src != "" && dst != "" {
					redirects = append(redirects, Redirect{src, dst})
					delete(domainsSet, src) // Remove redirect source from main domains
				}
			}
		}
	}

	// Convert domains set to slice
	for domain := range domainsSet {
		domains = append(domains, domain)
	}
	if len(domains) == 0 {
		return nil, fmt.Errorf("no valid domains remaining after redirect processing")
	}

	// Get SSL configuration
	sslConfig := getSSLConfig(labels)

	// Debug log the SSL configuration
	if logger.ShowDebug {
		logger.Debug("SSL config for %s: enabled=%v, provider=%s, email=%s",
			strings.TrimPrefix(c.Names[0], "/"),
			sslConfig.Enabled,
			sslConfig.Provider,
			sslConfig.Email)
	}

	return &ContainerConfig{
		Name:      strings.TrimPrefix(c.Names[0], "/"),
		IP:        ip,
		Port:      port,
		Domains:   domains,
		Redirects: redirects,
		SSL:       sslConfig,
	}, nil
}

// generateNginxConfig creates the nginx configuration file for a container
func generateNginxConfig(tempDir string, config *ContainerConfig) error {
	confName := filepath.Join(tempDir, fmt.Sprintf("%s.conf", config.Name))
	confFile, err := os.Create(confName)
	if err != nil {
		return fmt.Errorf("error creating nginx config file: %v", err)
	}
	defer confFile.Close()

	// Prepare template data
	templateData := map[string]interface{}{
		"MainHosts":   strings.Join(config.Domains, " "),
		"IP":          config.IP,
		"Port":        config.Port,
		"Redirects":   config.Redirects,
		"SSL":         config.SSL.Enabled,
		"SSLProvider": config.SSL.Provider,
		"SSLEmail":    config.SSL.Email,
		"Config":      config, // Pass entire config for advanced templating
	}

	// Execute template
	if err := nginxTemplate.Execute(confFile, templateData); err != nil {
		return fmt.Errorf("template execution failed: %v", err)
	}

	return nil
}

// ContainerConfig holds the processed configuration for a container
type ContainerConfig struct {
	Name      string
	IP        string
	Port      string
	Domains   []string
	Redirects []Redirect
	SSL       SSLConfig
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
	// Acquire lock
	lockFilePath := "/tmp/proxma.lock"
	lockFile, err := os.OpenFile(lockFilePath, os.O_CREATE|os.O_RDWR, 0644)
	if err != nil {
		logger.Error("🔒 Cannot create/open lock file: %v", err)
		return
	}
	defer lockFile.Close()
	err = syscall.Flock(int(lockFile.Fd()), syscall.LOCK_EX|syscall.LOCK_NB)
	if err != nil {
		logger.Warning("⚠️  Another configuration regeneration is running")
		return
	}
	defer syscall.Flock(int(lockFile.Fd()), syscall.LOCK_UN)

	if logger.ShowDebug {
		logger.Debug("Starting configuration update...")
	}

	// Clear any problematic configs first
	problematicConfigs, _ := filepath.Glob("/etc/nginx/conf.d/*.conf")
	for _, conf := range problematicConfigs {
		// Keep the default.conf if it exists
		if filepath.Base(conf) == "default.conf" {
			continue
		}

		// Remove other configs to start fresh
		os.Remove(conf)
		if logger.ShowDebug {
			logger.Debug("Removed potentially problematic config: %s", filepath.Base(conf))
		}
	}

	// Create a clean default config to ensure Nginx can start
	defaultConfig := `
server {
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
}
`
	defaultConfigPath := "/etc/nginx/conf.d/default.conf"
	if err := os.WriteFile(defaultConfigPath, []byte(defaultConfig), 0644); err != nil {
		logger.Error("Failed to write default config: %v", err)
	} else {
		logger.Debug("Created clean default config")
	}

	// Reload Nginx with just the default config to ensure it starts
	if err := exec.Command("nginx", "-s", "reload").Run(); err != nil {
		logger.Error("Failed to reload Nginx with default config: %v", err)
		// If we can't even start with default config, something is seriously wrong
		return
	}

	// Track existing configs
	existingConfigs, _ := filepath.Glob("/etc/nginx/conf.d/*.conf")
	existingMap := make(map[string]bool)
	for _, conf := range existingConfigs {
		existingMap[filepath.Base(conf)] = true
	}

	// Setup temp directory
	tempDir := "/tmp/nginx_conf_temp"
	os.RemoveAll(tempDir)
	if err := os.MkdirAll(tempDir, 0755); err != nil {
		logger.Error("Failed to create temp directory: %v", err)
		return
	}

	// Process containers
	containers, err := listContainers(cli)
	if err != nil {
		logger.Error("Failed to list containers: %v", err)
		return
	}

	var (
		configuredContainers  []ContainerInfo
		skippedContainers     = make(map[string]string)
		problematicContainers []ContainerError
		sslCertificates       []CertificateInfo
	)

	// Process each container
	for _, c := range containers {
		containerName := strings.TrimPrefix(c.Names[0], "/")
		if !hasProxmaLabels(c.Labels) {
			skippedContainers[containerName] = "no proxma labels"
			if logger.ShowDebug {
				logger.Debug("⏭️  Skipped %s: no proxma labels", containerName)
			}
			continue
		}

		config, err := processContainer(cli, c)
		if err != nil {
			problematicContainers = append(problematicContainers, ContainerError{
				Name: containerName, Error: err,
			})
			continue
		}

		// Check if SSL is enabled but certificates don't exist
		if config.SSL.Enabled {
			certPath := fmt.Sprintf("/etc/certificates/live/%s/fullchain.pem", config.Domains[0])
			if _, err := os.Stat(certPath); os.IsNotExist(err) {
				logger.Warning("SSL certificates not found for %s, will attempt to obtain", config.Domains[0])
			}
		}

		if err := generateNginxConfig(tempDir, config); err != nil {
			problematicContainers = append(problematicContainers, ContainerError{
				Name: containerName, Error: fmt.Errorf("nginx config failed: %v", err),
			})
			continue
		}

		// Store container info
		containerInfo := ContainerInfo{
			Name:        containerName,
			Domains:     config.Domains,
			Port:        config.Port,
			SSL:         config.SSL.Enabled,
			SSLEmail:    config.SSL.Email,
			SSLProvider: config.SSL.Provider,
			Redirects:   config.Redirects,
		}
		configuredContainers = append(configuredContainers, containerInfo)

		// Only track SSL certificates if SSL is enabled
		if config.SSL.Enabled {
			certInfo := CertificateInfo{
				Domain:   config.Domains[0],
				Status:   getCertificateStatus(config.Domains[0]),
				Provider: config.SSL.Provider,
			}
			sslCertificates = append(sslCertificates, certInfo)

			// Log SSL information in debug mode
			if logger.ShowDebug {
				logger.Debug("SSL certificate tracked for %s (status: %s)", certInfo.Domain, certInfo.Status)
			}
		}
	}

	// Identify changes
	var added, removed []string
	newConfigs, _ := filepath.Glob(filepath.Join(tempDir, "*.conf"))
	newMap := make(map[string]bool)
	for _, conf := range newConfigs {
		baseName := filepath.Base(conf)
		newMap[baseName] = true
		if !existingMap[baseName] {
			added = append(added, strings.TrimSuffix(baseName, ".conf"))
		}
	}
	for conf := range existingMap {
		if !newMap[conf] && conf != "default.conf" {
			removed = append(removed, strings.TrimSuffix(conf, ".conf"))
		}
	}

	// Log changes prominently
	if len(added) > 0 {
		logger.logSection("➕ CONTAINER ADDED")
		for _, name := range added {
			for _, container := range configuredContainers {
				if container.Name == name {
					logger.Success("✅ Added: %s", container.Name)
					logger.Info("   • Domains: %s", strings.Join(container.Domains, ", "))
					logger.Info("   • Port: %s", container.Port)
					if len(container.Redirects) > 0 {
						logger.Info("   • Redirects: %s", formatRedirects(container.Redirects))
					}
					if container.SSL {
						logger.Info("   • SSL: enabled")
						if container.SSLEmail != "" {
							logger.Info("   • SSL Email: %s", container.SSLEmail)
						}
						if container.SSLProvider != "" {
							logger.Info("   • SSL Provider: %s", container.SSLProvider)
						}
					}
				}
			}
		}
		fmt.Println()
	}

	if len(removed) > 0 {
		logger.logSection("➖ CONTAINER REMOVED")
		for _, name := range removed {
			logger.Info("❌ Removed: %s", name)
		}
		fmt.Println()
	}

	// Apply configuration
	output, err := exec.Command("nginx", "-t", "-c", "/etc/nginx/nginx.conf").CombinedOutput()
	if err != nil {
		logger.Error("Nginx configuration test failed: %s", output)
		return
	}

	if err := updateConfigFiles(tempDir, "/etc/nginx/conf.d"); err != nil {
		logger.Error("Failed to update configuration files: %v", err)
		return
	}

	if err := exec.Command("nginx", "-s", "reload").Run(); err != nil {
		logger.Error("Failed to reload Nginx: %v", err)
		return
	}

	// Display SSL certificate summary
	if len(sslCertificates) > 0 {
		logger.logSection("🔒 SSL Certificates")
		for _, cert := range sslCertificates {
			statusEmoji := "⏳"
			if cert.Status == "valid" {
				statusEmoji = "✅"
			} else if cert.Status == "error" {
				statusEmoji = "❌"
			}
			logger.Info("%s %s: %s (provider: %s)", statusEmoji, cert.Domain, cert.Status, cert.Provider)
		}
		fmt.Println()
	}

	// Handle SSL if enabled
	if len(sslCertificates) > 0 {
		// Check if any certificates need to be issued
		pendingCerts := false
		for _, cert := range sslCertificates {
			if cert.Status == "pending" {
				pendingCerts = true
				break
			}
		}

		if pendingCerts {
			logger.Info("🔒 Attempting to obtain SSL certificates...")
			issueSSLCertsIfMissing()

			// Reload Nginx after certificates are issued
			if err := exec.Command("nginx", "-s", "reload").Run(); err != nil {
				logger.Error("Failed to reload Nginx after certificate issuance: %v", err)
			} else {
				logger.Success("Nginx reloaded after certificate issuance")
			}
		}
	}

	// Show problems if any
	if len(problematicContainers) > 0 {
		logger.logSection("⚠️  PROBLEMS")
		for _, container := range problematicContainers {
			logger.Error("• %s: %v", container.Name, container.Error)
		}
		fmt.Println()
	}

	os.RemoveAll(tempDir)
}

func formatRedirects(redirects []Redirect) string {
	var formatted []string
	for _, r := range redirects {
		formatted = append(formatted, fmt.Sprintf("%s ➡️  %s", r.Source, r.Target))
	}
	return strings.Join(formatted, ", ")
}

func getCertificateStatus(domain string) string {
	certPath := fmt.Sprintf("/etc/certificates/live/%s/fullchain.pem", domain)
	if _, err := os.Stat(certPath); err == nil {
		return "valid"
	}

	// Check for pending/failed status
	if _, err := os.Stat(fmt.Sprintf("/tmp/cert_issue_failed_%s.marker", domain)); err == nil {
		return "error"
	}

	return "pending"
}

func issueSSLCertsIfMissing() {
	// Check if SSL is enabled either globally or for specific containers
	globalSSLEnabled, _ := strconv.ParseBool(getEnvOrDefault("PROXMA_SSL", "false"))

	// If global SSL is disabled, we still need to check if any containers have SSL enabled
	if !globalSSLEnabled {
		// Check if we have any pending certificates, which would indicate container-specific SSL
		pendingCerts := false
		confFiles, err := filepath.Glob("/etc/nginx/conf.d/*.conf")
		if err == nil {
			for _, confFile := range confFiles {
				content, err := os.ReadFile(confFile)
				if err == nil && strings.Contains(string(content), "SSL_PROVIDER:") {
					pendingCerts = true
					break
				}
			}
		}

		if !pendingCerts {
			log.Println("SSL is disabled globally and no containers have SSL enabled, skipping certificate checks.")
			return
		}

		log.Println("SSL is enabled for specific containers, proceeding with certificate checks.")
	}

	// Rest of the function remains the same
	confFiles, err := filepath.Glob("/etc/nginx/conf.d/*.conf")
	if err != nil {
		log.Println("Error finding conf files:", err)
		return
	}

	for _, confFile := range confFiles {
		content, err := os.ReadFile(confFile)
		if err != nil {
			log.Printf("Could not read conf file (%s): %v", confFile, err)
			continue
		}

		contentStr := string(content)
		if !strings.Contains(contentStr, "/.well-known/acme-challenge/") {
			continue
		}

		// Check if this is a development mode config
		isDevelopment := strings.Contains(contentStr, "SSL_PROVIDER: development")

		domains := extractDomainsFromConf(contentStr)
		if len(domains) == 0 {
			log.Printf("No domains found in conf file: %s", confFile)
			continue
		}

		primaryDomain := domains[0]
		certPath := "/etc/certificates/live/" + primaryDomain + "/fullchain.pem"

		// For development mode, generate self-signed certificates
		if isDevelopment {
			if _, err := os.Stat(certPath); err == nil {
				log.Printf("Self-signed certificate already exists for domain %s.", primaryDomain)
				continue
			}

			log.Printf("🔧 Generating self-signed certificate for development: %v", domains)

			// Create directory structure
			certDir := filepath.Dir(certPath)
			if err := os.MkdirAll(certDir, 0755); err != nil {
				log.Printf("Failed to create certificate directory: %v", err)
				continue
			}

			// Generate self-signed certificate
			keyPath := filepath.Join(certDir, "privkey.pem")
			cmd := exec.Command("openssl", "req", "-x509", "-nodes", "-newkey", "rsa:2048",
				"-keyout", keyPath, "-out", certPath, "-days", "365",
				"-subj", fmt.Sprintf("/CN=%s", primaryDomain))

			out, err := cmd.CombinedOutput()
			if err != nil {
				log.Printf("🚨 Failed to generate self-signed certificate: %v\nOutput: %s", err, string(out))
				continue
			}

			log.Printf("✅ Generated self-signed certificate for: %v", domains)
			continue
		}

		// For production mode, use Certbot/Let's Encrypt
		if _, err := os.Stat(certPath); err == nil {
			log.Printf("SSL certificate already exists for domain %s.", primaryDomain)
			continue
		}

		// Extract email from config
		email := ""
		if strings.Contains(contentStr, "SSL_EMAIL:") {
			lines := strings.Split(contentStr, "\n")
			for _, line := range lines {
				if strings.Contains(line, "SSL_EMAIL:") {
					parts := strings.SplitN(line, ":", 2)
					if len(parts) == 2 {
						email = strings.TrimSpace(parts[1])
						break
					}
				}
			}
		}

		// Fall back to global email if not found in config
		if email == "" {
			email = getEnvOrDefault("PROXMA_SSL_EMAIL", "")
		}

		if email == "" {
			log.Printf("🚨 No SSL email found for %s. SSL certificate issuance will be skipped.", primaryDomain)
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
