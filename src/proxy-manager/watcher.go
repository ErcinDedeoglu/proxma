package main

import (
	"bufio"
	"log"
	"os/exec"

	"github.com/docker/docker/client"
)

func watchDockerEvents(cli *client.Client) {
	cmd := exec.Command("docker", "events", "--filter", "type=container", "--filter", "event=start", "--filter", "event=die", "--filter", "event=stop")
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		log.Fatalf("Error getting docker events stdout: %v", err)
	}
	if err := cmd.Start(); err != nil {
		log.Fatalf("Error starting docker events cmd: %v", err)
	}

	scanner := bufio.NewScanner(stdout)
	for scanner.Scan() {
		go generateConfigs(cli)
	}

	if err := scanner.Err(); err != nil {
		log.Fatalf("Docker events reading error: %v", err)
	}
}
