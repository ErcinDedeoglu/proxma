#!/bin/sh
# Run logrotate and log the result
logrotate /etc/logrotate.d/proxma-logrotate.conf
echo "[$(date)] Logrotate executed" >> /var/proxma/logs/logrotate.log