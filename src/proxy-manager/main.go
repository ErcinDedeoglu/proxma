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

var nginxTemplate = template.Must(template.New("nginx").Parse(`
server {
    listen 80;
    server_name {{ .Hosts }};
    location / {
        proxy_pass http://{{ .IP }}:{{ .Port }};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
`))

// Update struct to include Hosts (string with spaces)
type NginxConf struct {
	Hosts string
	IP    string
	Port  string
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

		if hasHosts && hasPort {
			inspect, err := cli.ContainerInspect(ctx, c.ID)
			if err != nil {
				log.Printf("Failed inspecting container %v: %v", c.ID, err)
				continue
			}
			ip := ""
			for _, net := range inspect.NetworkSettings.Networks {
				ip = net.IPAddress
				break
			}
			if ip == "" {
				continue
			}

			// generate config filename based on container name or ID
			confName := fmt.Sprintf("/etc/nginx/conf.d/%s.conf", c.Names[0][1:]) // strip leading "/"
			conf, err := os.Create(confName)
			if err != nil {
				log.Printf("Error creating config: %v", err)
				continue
			}

			// Replace comma with space for nginx "server_name"
			serverNames := strings.ReplaceAll(hostsLabel, ",", " ")

			nginxTemplate.Execute(conf, NginxConf{Hosts: serverNames, IP: ip, Port: port})
			conf.Close()
			activeConfs[confName] = true

			log.Printf("Configured hosts [%s] -> %s:%s", serverNames, ip, port)
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
