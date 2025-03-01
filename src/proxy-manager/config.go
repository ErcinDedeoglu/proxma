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
	globalEnabled, _ := strconv.ParseBool(getEnvOrDefault("PROXMA_SSL", "false"))
	sslEnabled := globalEnabled
	if val, exists := labels["proxma.ssl"]; exists {
		containerEnabled, err := strconv.ParseBool(val)
		if err == nil {
			sslEnabled = containerEnabled
		}
	}

	provider := strings.ToLower(getEnvOrDefault("PROXMA_SSL_PROVIDER", "letsencrypt"))
	if val, exists := labels["proxma.ssl.provider"]; exists && val != "" {
		provider = strings.ToLower(val)
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

	return SSLConfig{
		Enabled:       sslEnabled,
		Provider:      provider,
		Email:         email,
		ExtraSettings: extras,
	}
}
