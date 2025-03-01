#!/bin/bash
set -e

# Prepare required directories explicitly
mkdir -p /var/www/certbot
mkdir -p /etc/nginx/conf.d
mkdir -p /etc/certificates
mkdir -p /var/log/nginx
mkdir -p /var/run/nginx

# Set proper permissions
chown -R nginx:nginx /var/www/certbot
chown -R nginx:nginx /etc/nginx/conf.d
chown -R nginx:nginx /var/log/nginx
chown -R nginx:nginx /var/run/nginx

# Cleanup existing configs clearly
rm -rf /etc/nginx/conf.d/*.conf

# Test nginx configuration
echo "🔍 Testing nginx configuration..."
nginx -t -v

# Run proxy-manager once initially to generate basic configurations rapidly
echo "🔧 Running initial proxy-manager configuration..."
proxy-manager & sleep 3 && kill $!

# Setup cronjobs explicitly for cert renewal and retry clearly
echo "🕑 Configuring cronjobs for SSL renewal and retries..."
{
    # Daily certificate renewal cron
    echo "0 2 * * * certbot renew --webroot -w /var/www/certbot \
          --config-dir /etc/certificates \
          --work-dir /etc/certificates/work \
          --logs-dir /etc/certificates/log \
          --post-hook='nginx -s reload' >> /var/log/cert_renew.log 2>&1"
    # Hourly SSL issuance retry cron
    echo "0 * * * * /usr/local/bin/proxy-manager-retry.sh >> /var/log/cert_retry.log 2>&1"
} | crontab -

# Clearly ensure retry script is executable
chmod +x /usr/local/bin/proxy-manager-retry.sh

# Finally, supervise nginx and proxy-manager in the foreground explicitly
echo "🎯 Starting supervisord clearly in foreground mode..."
exec supervisord -c /etc/supervisor/conf.d/supervisor.conf