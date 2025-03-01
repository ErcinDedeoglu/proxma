package main

import (
	"bufio"
	"context"
	"fmt"
	"log"
	"os"
	"os/exec"
	"strings"
	"text/template"

	"github.com/docker/docker/api/types/container"
	"github.com/docker/docker/client"
)

var nginxTemplate = template.Must(template.New("nginx").Funcs(template.FuncMap{
	"split": strings.Split,
}).Parse(`

{{range .Redirects}}
server {
    listen 80;
    server_name {{.Source}};
    return 301 {{if $.SSL}}https{{else}}http{{end}}://{{.Target}}$request_uri;
}
{{end}}

server {
    listen 80;
    server_name {{ .MainHosts }};
    
    {{if .SSL}}
    location /.well-known/acme-challenge/ {
        root /var/www/certbot;
    }
    location / {
        return 301 https://$host$request_uri;
    }
    {{else}}
    location / {
        proxy_pass http://{{ .IP }}:{{ .Port }};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
    {{end}}
}

{{if .SSL}}
server {
    listen 443 ssl;
    server_name {{ .MainHosts }};
    
    ssl_certificate /etc/letsencrypt/live/{{(index (split .MainHosts " ") 0)}}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/{{(index (split .MainHosts " ") 0)}}/privkey.pem;
    
    location / {
        proxy_pass http://{{ .IP }}:{{ .Port }};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
{{end}}

`))

type NginxConf struct {
	MainHosts string
	IP        string
	Port      string
	Redirects []Redirect
	SSL       bool
}

type Redirect struct {
	Source string
	Target string
}

func generateConfigs(cli *client.Client) {
	ctx := context.Background()
	containers, err := cli.ContainerList(ctx, container.ListOptions{})
	if err != nil {
		log.Printf("Error listing containers: %v", err)
		return
	}
	os.MkdirAll("/etc/nginx/conf.d", 0755)
	activeConfs := make(map[string]bool)

	for _, c := range containers {
		labels := c.Labels
		hostsLabel, hasHosts := labels["proxma.hosts"]
		port, hasPort := labels["proxma.port"]
		redirectsLabel, hasRedirects := labels["proxma.redirects"]
		sslEnabled := false
		if labels["proxma.ssl"] == "true" {
			sslEnabled = true
		}

		if hasHosts && hasPort {
			var ip string
			inspect, err := cli.ContainerInspect(ctx, c.ID)
			if err != nil {
				log.Printf("Failed inspecting container %v: %v", c.ID, err)
				continue
			}

			for _, net := range inspect.NetworkSettings.Networks {
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
							redirects = append(redirects, Redirect{Source: src, Target: dst})
							// Remove redirect source from main hosts if present
							delete(mainHostsSet, src)
						}
					}
				}
			}

			// Remaining hosts after removing redirect sources
			mainHostsSlice := []string{}
			for host := range mainHostsSet {
				mainHostsSlice = append(mainHostsSlice, host)
			}

			// Generate config filename based on container name
			confName := fmt.Sprintf("/etc/nginx/conf.d/%s.conf", c.Names[0][1:])
			conf, err := os.Create(confName)
			if err != nil {
				log.Printf("Error creating config: %v", err)
				continue
			}

			nginxTemplate.Execute(conf, NginxConf{
				MainHosts: strings.Join(mainHostsSlice, " "),
				IP:        ip,
				Port:      port,
				Redirects: redirects,
				SSL:       sslEnabled,
			})

			conf.Close()
			activeConfs[confName] = true
			log.Printf("Configured hosts [%s] with redirects [%v] -> %s:%s", strings.Join(mainHostsSlice, " "), redirects, ip, port)
		}
	}

	// Clean stale configurations
	files, err := os.ReadDir("/etc/nginx/conf.d")
	if err != nil {
		log.Printf("Error reading directory: %v", err)
		return
	}
	for _, file := range files {
		fullPath := "/etc/nginx/conf.d/" + file.Name()
		if !activeConfs[fullPath] {
			os.Remove(fullPath)
			log.Printf("Removed stale config: %s", file.Name())
		}
	}

	// Reload nginx
	if err := exec.Command("nginx", "-s", "reload").Run(); err != nil {
		log.Printf("Failed to reload nginx: %v", err)
	} else {
		log.Printf("Nginx reloaded successfully")
	}
}

func watchDockerEvents(cli *client.Client) {
	cmd := exec.Command("docker", "events", "--filter", "type=container", "--filter", "event=start", "--filter", "event=die", "--filter", "event=stop")
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		log.Fatalf("Error creating stdout pipe: %v", err)
	}
	if err := cmd.Start(); err != nil {
		log.Fatalf("Error starting docker events command: %v", err)
	}
	scanner := bufio.NewScanner(stdout)
	for scanner.Scan() {
		event := scanner.Text()
		log.Printf("Docker event: %s", event)
		go generateConfigs(cli)
	}
	if err := scanner.Err(); err != nil {
		log.Fatalf("Error reading docker events: %v", err)
	}
	if err := cmd.Wait(); err != nil {
		log.Fatalf("Error waiting for docker events command: %v", err)
	}
}

func main() {
	log.Println("proxma proxy-manager started")
	cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
	if err != nil {
		log.Fatalf("Failed to create Docker client: %v", err)
	}
	defer cli.Close()

	generateConfigs(cli)
	watchDockerEvents(cli)
}
