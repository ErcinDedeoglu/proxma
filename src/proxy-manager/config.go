package main

import (
	"os"
	"strconv"
	"strings"
)

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

type SSLConfig struct {
	Enabled       bool
	Provider      string
	Email         string
	ExtraSettings map[string]string
}

func getEnvOrDefault(key, def string) string {
	val := os.Getenv(key)
	if val == "" {
		return def
	}
	return val
}

func getSSLConfig(labels map[string]string) SSLConfig {
	// First check global SSL setting
	globalEnabled, _ := strconv.ParseBool(getEnvOrDefault("PROXMA_SSL", "false"))

	// Debug log the global SSL setting
	if logger.ShowDebug {
		logger.Debug("Global SSL setting: %v", globalEnabled)
	}

	if !globalEnabled {
		// If global SSL is disabled, check container-specific override
		if val, exists := labels["proxma.ssl"]; exists {
			containerEnabled, err := strconv.ParseBool(val)
			if err == nil && containerEnabled {
				globalEnabled = true
				if logger.ShowDebug {
					logger.Debug("Container-specific SSL override: enabled")
				}
			}
		}
	}

	// If we get here and SSL is still disabled, return early
	if !globalEnabled {
		return SSLConfig{
			Enabled:       false,
			Provider:      "",
			Email:         "",
			ExtraSettings: make(map[string]string),
		}
	}

	// If we get here, SSL is enabled (either globally or container-specific)
	sslEnabled := true

	provider := strings.ToLower(getEnvOrDefault("PROXMA_SSL_PROVIDER", "letsencrypt"))
	if val, exists := labels["proxma.ssl.provider"]; exists && val != "" {
		provider = strings.ToLower(val)
	}

	// Check for development mode
	devMode := false
	if provider == "development" || provider == "self-signed" || provider == "dev" {
		devMode = true
		provider = "development"
	}

	email := getEnvOrDefault("PROXMA_SSL_EMAIL", "")
	if val, exists := labels["proxma.ssl.email"]; exists && val != "" {
		email = val
	}

	extras := make(map[string]string)
	for k, v := range labels {
		if strings.HasPrefix(k, "proxma.ssl.") && k != "proxma.ssl" && k != "proxma.ssl.provider" && k != "proxma.ssl.email" {
			extras[k] = v
		}
	}

	// Add development mode flag to extras
	if devMode {
		extras["development_mode"] = "true"
	}

	return SSLConfig{
		Enabled:       sslEnabled,
		Provider:      provider,
		Email:         email,
		ExtraSettings: extras,
	}
}
