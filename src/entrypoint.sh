#!/bin/bash
set -e

# Setup SSL renewal cron jobs
echo "🕐 Setting up SSL renewal cron jobs..."
{
    echo "0 2 * * * certbot renew --webroot -w /var/www/certbot \
          --config-dir /etc/certificates \
          --work-dir /etc/certificates/work \
          --logs-dir /etc/certificates/log \
          --post-hook='nginx -s reload' >> /var/log/cert_renew.log 2>&1"
    echo "0 * * * * /usr/local/bin/proxy-manager-retry.sh >> /var/log/cert_retry.log 2>&1"
} | crontab -

# Start services
echo "🚀 Starting proxma services..."
exec supervisord -c /etc/supervisor/conf.d/supervisor.conf