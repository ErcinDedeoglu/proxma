#!/bin/bash
set -e
mkdir -p /var/www/certbot
rm -f /etc/nginx/conf.d/*.conf

# Initial proxy config generation
proxy-manager & sleep 3 && kill $!

# Start nginx temporarily (no SSL) to allow webroot challenges to succeed
sed -i '/ssl_certificate/d; /listen 443 ssl/d; /return 301 https/d' /etc/nginx/conf.d/*.conf
nginx

# Issue certificates based on SSL config per host
for conf in /etc/nginx/conf.d/*.conf; do
  if grep -q '/.well-known/acme-challenge/' "$conf"; then
    DOMAIN=$(grep server_name "$conf" | head -1 | awk '{print $2}' | tr -d ';')
    
    # Fetch SSL provider from conf by matching a comment/placeholder in generated config
    PROVIDER=$(grep "# SSL_PROVIDER:" "$conf" | awk '{print tolower($3)}')
    EMAIL=$(grep "# SSL_EMAIL:" "$conf" | awk '{print $3}')

    ACME_SERVER="https://acme-v02.api.letsencrypt.org/directory"
    if [ "$PROVIDER" == "zerossl" ]; then
      ACME_SERVER="https://acme.zerossl.com/v2/DV90"
    elif [ "$PROVIDER" == "buypass" ]; then
      ACME_SERVER="https://api.buypass.com/acme/directory"
    fi

    # Obtain certificate
    if [ ! -d "/etc/letsencrypt/live/$DOMAIN" ]; then
      certbot certonly --webroot -w /var/www/certbot \
        --server "$ACME_SERVER" \
        --agree-tos --non-interactive \
        --email "$EMAIL" \
        $(grep server_name "$conf" | head -1 | awk '{for(i=2;i<=NF;i++)print "-d "$i}' | tr -d ';')
    fi
  fi
done

# Regenerate full configs with SSL
rm -f /etc/nginx/conf.d/*.conf
proxy-manager & sleep 3 && kill $!
nginx -s reload

# Renew cronjob stays same (certbot handles renewal based on stored metadata)
echo "0 2 * * * certbot renew --webroot -w /var/www/certbot --post-hook='nginx -s reload'" | crontab -
cron -f &

exec supervisord -c /etc/supervisor/conf.d/supervisor.conf