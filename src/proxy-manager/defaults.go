package main

import (
	"fmt"
	"os"
)

// Default HTML content for the landing page
const defaultPageHTML = `<!DOCTYPE html>
<html>
<head>
    <title>proxma Proxy Manager</title>
    <style>
        body {font-family: Arial, sans-serif; text-align: center; margin-top: 40px;}
        h1 {color: #337ab7;}
        p {color: #444;}
        pre {background:#f4f4f4; padding:10px; margin:10px auto; width:fit-content;}
        a.button {
            background:#337ab7; color:white; padding:7px 14px;
            border-radius:4px; text-decoration:none; display:inline-block; margin-top:10px;}
    </style>
</head>
<body>
    <h1>🚀 Welcome to proxma!</h1>
    <p>You're seeing this default landing page because no explicit proxma domain configuration was found yet.</p>
    <p><strong>To set up your own container</strong>, simply add labels explicitly:</p>
    <pre>
proxma.hosts: "example.com,www.example.com"
proxma.port: "80"
    </pre>
    <a class="button" href="https://github.com/ercindedeoglu/proxma">proxma Documentation</a>
    <p style="margin-top:60px;color:#888;">Powered by proxma Docker Proxy Manager</p>
</body>
</html>`

// Default nginx configuration
const defaultNginxConfig = `server {
    listen 80 default_server;
    listen [::]:80 default_server;
    
    server_name _;
    root /var/www/default_page;
    
    location / {
        try_files $uri $uri/ =404;
    }

    location /.well-known/acme-challenge/ {
        root /var/www/certbot;
    }
}`

// Function to ensure all default files and directories exist
func ensureDefaults() error {
	// Create required directories
	dirs := []string{
		"/var/www/default_page",
		"/var/www/certbot",
		"/etc/nginx/conf.d",
		"/etc/certificates",
		"/var/log/nginx",
		"/var/run/nginx",
	}

	for _, dir := range dirs {
		if err := os.MkdirAll(dir, 0755); err != nil {
			return fmt.Errorf("failed to create directory %s: %v", dir, err)
		}
		logger.Debug("Created directory: %s", dir)
	}

	// Set proper permissions
	if err := os.Chown("/var/www/certbot", 101, 102); err != nil { // 101 and 102 are nginx user and group IDs
		logger.Warning("Failed to set certbot directory permissions: %v", err)
	}

	// Write default page
	defaultPagePath := "/var/www/default_page/index.html"
	if err := os.WriteFile(defaultPagePath, []byte(defaultPageHTML), 0644); err != nil {
		return fmt.Errorf("failed to write default page: %v", err)
	}
	logger.Debug("Created default page: %s", defaultPagePath)

	// Write default nginx config
	defaultConfigPath := "/etc/nginx/conf.d/default.conf"
	if err := os.WriteFile(defaultConfigPath, []byte(defaultNginxConfig), 0644); err != nil {
		return fmt.Errorf("failed to write default nginx config: %v", err)
	}
	logger.Debug("Created default nginx config: %s", defaultConfigPath)

	return nil
}
