#!/bin/bash
set -e

mkdir -p /var/www/certbot
rm -f /etc/nginx/conf.d/*.conf

# Run proxy-manager to generate initial configs (no SSL)
proxy-manager & sleep 3 && kill $!

# Temporarily start nginx to issue certificates via certbot (stop after)
sed -i '/ssl_certificate/d; /listen 443 ssl/d; /return 301 https/d' /etc/nginx/conf.d/*.conf
nginx && sleep 5 && nginx -s stop

# Issue certificates based on generated nginx configs
for conf in /etc/nginx/conf.d/*.conf; do
  if grep -q '/.well-known/acme-challenge/' "$conf"; then
    DOMAIN=$(grep server_name "$conf" | head -1 | awk '{print $2}' | tr -d ';')

    PROVIDER=$(grep "# SSL_PROVIDER:" "$conf" | awk '{print tolower($3)}')
    EMAIL=$(grep "# SSL_EMAIL:" "$conf" | awk '{print $3}')
    ACME_SERVER="https://acme-v02.api.letsencrypt.org/directory"
    if [ "$PROVIDER" == "zerossl" ]; then
      ACME_SERVER="https://acme.zerossl.com/v2/DV90"
    elif [ "$PROVIDER" == "buypass" ]; then
      ACME_SERVER="https://api.buypass.com/acme/directory"
    fi

    if [ ! -d "/etc/letsencrypt/live/$DOMAIN" ]; then
      certbot certonly --webroot -w /var/www/certbot \
        --server "$ACME_SERVER" \
        --agree-tos --non-interactive \
        --email "$EMAIL" \
        $(grep server_name "$conf" | head -1 | awk '{for(i=2;i<=NF;i++)print "-d "$i}' | tr -d ';')
    fi
  fi
done

# Now regenerate nginx configs to enable SSL fully
rm -f /etc/nginx/conf.d/*.conf
proxy-manager & sleep 3 && kill $!

# DO NOT start nginx again manually here, supervisord will start it!
# Setup cron job for renewals clearly
echo "0 2 * * * certbot renew --webroot -w /var/www/certbot --post-hook='nginx -s reload'" | crontab -

# Finally start supervisord (it will manage cron, nginx, proxy-manager)
exec supervisord -c /etc/supervisor/conf.d/supervisor.conf