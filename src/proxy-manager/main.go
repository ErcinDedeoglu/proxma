package main

import (
	"log"

	"github.com/docker/docker/client"
)

func main() {
	log.Println("proxma proxy-manager started")

	cli, err := client.NewClientWithOpts(client.FromEnv, client.WithAPIVersionNegotiation())
	if err != nil {
		log.Fatalf("Failed Docker client creation: %v", err)
	}
	defer cli.Close()

	generateConfigs(cli)
	watchDockerEvents(cli)
}
