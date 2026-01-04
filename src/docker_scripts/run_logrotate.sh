#!/bin/sh
# Run logrotate with state file to track rotation times
# The -v flag provides verbose output for debugging
# State file ensures logrotate tracks when files were last rotated

STATE_FILE="/var/proxma/logs/logrotate.state"

# Run logrotate (not forced - let it decide based on size/time)
logrotate -v -s "$STATE_FILE" /etc/logrotate.d/proxma-logrotate.conf >> /var/proxma/logs/logrotate.log 2>&1
echo "[$(date)] Logrotate check completed" >> /var/proxma/logs/logrotate.log