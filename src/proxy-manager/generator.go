package main

import (
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/docker/docker/client"
)

func generateConfigs(cli *client.Client) {
	containers, err := listContainers(cli)
	if err != nil {
		log.Printf("Error listing containers: %v", err)
		return
	}

	os.MkdirAll("/etc/nginx/conf.d", 0755)
	activeConfs := make(map[string]bool)

	for _, c := range containers {
		labels := c.Labels
		sslConfig := getSSLConfig(labels)
		hostsLabel, hasHosts := labels["proxma.hosts"]
		port, hasPort := labels["proxma.port"]
		redirectsLabel, hasRedirects := labels["proxma.redirects"]

		if !(hasHosts && hasPort) {
			continue
		}

		containerDetails, err := inspectContainer(cli, c.ID)
		if err != nil {
			log.Printf("Failed inspecting container %v: %v", c.ID, err)
			continue
		}

		ip := ""
		for _, net := range containerDetails.NetworkSettings.Networks {
			ip = net.IPAddress
			break
		}
		if ip == "" {
			continue
		}

		mainHostsSet := make(map[string]bool)
		for _, h := range strings.Split(hostsLabel, ",") {
			mainHostsSet[strings.TrimSpace(h)] = true
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

		mainHostsSlice := []string{}
		for host := range mainHostsSet {
			mainHostsSlice = append(mainHostsSlice, host)
		}

		confName := "/etc/nginx/conf.d/" + c.Names[0][1:] + ".conf"
		confFile, err := os.Create(confName)
		if err != nil {
			continue
		}
		nginxTemplate.Execute(confFile, map[string]interface{}{
			"MainHosts":   strings.Join(mainHostsSlice, " "),
			"IP":          ip,
			"Port":        port,
			"Redirects":   redirects,
			"SSL":         sslConfig.Enabled,
			"SSLProvider": sslConfig.Provider,
			"SSLEmail":    sslConfig.Email,
		})
		confFile.Close()
		activeConfs[confName] = true
	}

	files, _ := os.ReadDir("/etc/nginx/conf.d")
	for _, file := range files {
		fullPath := "/etc/nginx/conf.d/" + file.Name()
		if !activeConfs[fullPath] {
			os.Remove(fullPath)
		}
	}

	exec.Command("nginx", "-s", "reload").Run()

	issueSSLCertsIfMissing()
}

func issueSSLCertsIfMissing() {
	confFiles, err := filepath.Glob("/etc/nginx/conf.d/*.conf")
	if err != nil {
		log.Println("Error finding conf files:", err)
		return
	}

	for _, confFile := range confFiles {
		content, err := os.ReadFile(confFile)
		if err != nil {
			continue
		}

		if strings.Contains(string(content), "/.well-known/acme-challenge/") {
			// extract domain(s) from conf
			domain := extractDomainFromConf(string(content))
			certPath := "/etc/certificates/live/" + domain + "/fullchain.pem"
			if _, err := os.Stat(certPath); os.IsNotExist(err) {
				// certificate missing, trigger certbot here
				log.Printf("Issuing SSL Certificate for %s", domain)

				cmd := exec.Command("certbot",
					"certonly", "--webroot", "-w", "/var/www/certbot",
					"--agree-tos", "--non-interactive",
					"--config-dir", "/etc/certificates",
					"--email", os.Getenv("PROXMA_SSL_EMAIL"), // from your global environment
					"-d", domain)

				out, err := cmd.CombinedOutput()
				if err != nil {
					log.Printf("Certbot error (%s): %s", domain, string(out))
					continue
				} else {
					log.Printf("Certbot success for (%s): %s", domain, string(out))
					// reload nginx after successful certbot run
					exec.Command("nginx", "-s", "reload").Run()
				}
			}
		}
	}
}

// Quick simplified example:
func extractDomainFromConf(conf string) string {
	// parse conf to get domain
	lines := strings.Split(conf, "\n")
	for _, line := range lines {
		if strings.Contains(line, "server_name") {
			fields := strings.Fields(line)
			if len(fields) >= 2 {
				return strings.TrimSuffix(fields[1], ";")
			}
		}
	}
	return ""
}
