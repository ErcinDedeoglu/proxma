package main

import (
	"bufio"
	"os"
	"os/exec"

	"github.com/docker/docker/client"
)

func watchDockerEvents(cli *client.Client) {
	logger.Info("Starting Docker events watcher...")

	cmd := exec.Command("docker", "events", "--filter", "type=container", "--filter", "event=start", "--filter", "event=die", "--filter", "event=stop")
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		logger.Error("Failed to get docker events stdout: %v", err)
		os.Exit(1)
	}

	if err := cmd.Start(); err != nil {
		logger.Error("Failed to start docker events command: %v", err)
		os.Exit(1)
	}

	logger.Success("Docker events watcher started")

	scanner := bufio.NewScanner(stdout)
	for scanner.Scan() {
		logger.Debug("Docker event received, triggering configuration update")
		go generateConfigs(cli)
	}

	if err := scanner.Err(); err != nil {
		logger.Error("Docker events reading error: %v", err)
		os.Exit(1)
	}
}
