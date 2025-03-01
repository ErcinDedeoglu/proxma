#!/bin/bash
set -e

mkdir -p /var/www/certbot

# Cleanup old configs safely to start from scratch
rm -f /etc/nginx/conf.d/*.conf

# Initial configs generation (proxy-manager first run)
proxy-manager & sleep 3 && kill $!

# Temporarily prevent any SSL configurations (remove SSL references)
sed -i '/ssl_certificate/d; /listen 443 ssl/d; /return 301 https/d' /etc/nginx/conf.d/*.conf

# Start nginx safely with HTTP-only for certbot
nginx

# Run certbot to create SSL certificates (but only for hosts flagged for SSL)
for conf in /etc/nginx/conf.d/*.conf; do
    if grep -q '/.well-known/acme-challenge/' "$conf"; then
        DOMAIN=$(grep server_name "$conf" | head -1 | awk '{print $2}' | tr -d ';')
        if [ ! -d "/etc/letsencrypt/live/$DOMAIN" ]; then
            certbot certonly --webroot -w /var/www/certbot \
                --agree-tos --non-interactive \
                --email your-email@example.com \
                -d $DOMAIN $(grep server_name "$conf" | head -1 | awk '{for(i=3;i<=NF;i++)print "-d "$i}' | tr -d ';')
        fi
    fi
done

# Fully regenerate configs cleanly after certs issued
rm -f /etc/nginx/conf.d/*.conf
proxy-manager & sleep 3 && kill $!

# Reload nginx with final configs (including SSL clearly)
nginx -s reload

# Start cron for certbot renewals
echo "0 2 * * * certbot renew --webroot -w /var/www/certbot --post-hook='nginx -s reload'" | crontab -
cron -f &

# Finally start supervisord properly
exec supervisord -c /etc/supervisor/conf.d/supervisor.conf