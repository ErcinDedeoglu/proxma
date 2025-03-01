package main

import (
	"os"

	"github.com/docker/docker/client"
)

func main() {
	// Initialize logger with debug mode based on environment variable
	debug := getEnvOrDefault("PROXMA_DEBUG", "false") == "true"
	logger = NewLogger(debug)

	logger.Info("Starting proxma proxy-manager %s", getVersion()) // Add version info

	// Create Docker client first
	cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
	if err != nil {
		logger.Error("Failed Docker client creation: %v", err)
		os.Exit(1)
	}
	defer cli.Close()

	// Ensure default files and directories exist
	if err := ensureDefaults(); err != nil {
		logger.Error("Failed to ensure defaults: %v", err)
		os.Exit(1)
	}
	logger.Success("Initialization completed")

	// Start the main processing
	processLoop(cli)
}

func processLoop(cli *client.Client) {
	// Initial configuration generation
	generateConfigs(cli)

	// Watch for Docker events
	watchDockerEvents(cli)
}
