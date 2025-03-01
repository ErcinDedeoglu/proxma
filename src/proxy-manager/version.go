package main

import "fmt"

const (
	Version   = "1.0.0"
	BuildTime = "2025-03-01"
)

func getVersion() string {
	return fmt.Sprintf("v%s (built: %s)", Version, BuildTime)
}
