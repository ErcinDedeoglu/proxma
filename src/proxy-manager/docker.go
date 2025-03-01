package main

import (
	"context"

	"github.com/docker/docker/api/types/container"
	"github.com/docker/docker/client"
)

// inspectContainer retrieves detailed information about a container by ID
func inspectContainer(cli *client.Client, id string) (container.InspectResponse, error) {
	ctx := context.Background()
	return cli.ContainerInspect(ctx, id)
}

// listContainers retrieves a list of running containers
func listContainers(cli *client.Client) ([]container.Summary, error) {
	ctx := context.Background()
	return cli.ContainerList(ctx, container.ListOptions{
		All: false, // Set to true if you want all containers (including stopped)
	})
}
