package main

import (
	"strings"
	"text/template"
)

var nginxTemplate = template.Must(template.New("nginx").Funcs(template.FuncMap{
	"split": strings.Split,
}).Parse(`
# SSL_PROVIDER: {{.SSLProvider}}
# SSL_EMAIL: {{.SSLEmail}}
{{range .Redirects}}
server {
    listen 80;
    server_name {{.Source}};
    return 301 {{if $.SSL}}https{{else}}http{{end}}://{{.Target}}$request_uri;
}
{{end}}
server {
    listen 80;
    server_name {{ .MainHosts }};
    
    {{if .SSL}}
    location /.well-known/acme-challenge/ {
        root /var/www/certbot;
    } 
    location / {
        return 301 https://$host$request_uri;
    }
    {{else}}
    location / {
        proxy_pass http://{{ .IP }}:{{ .Port }};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
    {{end}}
}
{{if .SSL}}
server {
    listen 443 ssl;
    server_name {{ .MainHosts }};
    
    ssl_certificate /etc/certificates/live/{{(index (split .MainHosts " ") 0)}}/fullchain.pem;
    ssl_certificate_key /etc/certificates/live/{{(index (split .MainHosts " ") 0)}}/privkey.pem;
    
    location / {
        proxy_pass http://{{ .IP }}:{{ .Port }};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
{{end}}
`))
