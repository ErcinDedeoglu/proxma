#!/bin/bash
set -e
mkdir -p /var/www/certbot
rm -f /etc/nginx/conf.d/*.conf

# Initially generate empty config (no cert requirement initially)
proxy-manager & sleep 3 && kill $!

# Start nginx initially
nginx

# setup renewal cronjob
echo "0 2 * * * certbot renew --webroot -w /var/www/certbot \
 --config-dir /etc/certificates --work-dir /etc/certificates/work \
 --logs-dir /etc/certificates/log \
 --post-hook='nginx -s reload'" | crontab -

exec supervisord -c /etc/supervisor/conf.d/supervisor.conf