#!/bin/sh
# Check if fail2ban should be disabled globally
if [ "$PROXMA_DISABLE_FAIL2BAN" = "true" ]; then
    echo "[$(date)] Fail2ban disabled via PROXMA_DISABLE_FAIL2BAN environment variable"
else
    # Start fail2ban with nginx backend instead of iptables
    fail2ban-client -x start
    echo "[$(date)] Fail2ban started"
fi

# Start cron daemon
crond -b -l 8
echo "[$(date)] Cron daemon started"

# Start Nginx
nginx -g "daemon off;" &
NGINX_PID=$!
echo "[$(date)] Nginx started with PID $NGINX_PID"

# Start the main application
exec /usr/local/bin/proxma