#!/bin/sh
# Update Cloudflare IPs and reload Nginx
/usr/local/bin/fetch_cloudflare_ips.sh
nginx -s reload
echo "[$(date)] Cloudflare IPs updated and Nginx reloaded" >> /var/proxma/logs/cloudflare_ip_updates.log