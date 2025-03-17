#!/bin/sh
# Start cron daemon
crond -b -l 8
echo "[$(date)] Cron daemon started"

# Start Nginx
nginx -g "daemon off;" &
NGINX_PID=$!
echo "[$(date)] Nginx started with PID $NGINX_PID"

# Start the main application
exec /usr/local/bin/proxma