#!/bin/sh

# Clean up old numbered logrotate files (legacy format) on startup
# New config uses dateext format, old .1, .2.gz files won't be managed
find /var/proxma/logs -name "*.log.[0-9]*" -type f -delete 2>/dev/null || true

# Function to check if IPv6 is supported
check_ipv6_support() {
    # Try to create an IPv6 socket
    if nc -6 -z -w1 ::1 1 2>/dev/null; then
        return 0
    else
        # Alternative check: look at /proc/net/if_inet6
        if [ -s /proc/net/if_inet6 ]; then
            return 0
        else
            return 1
        fi
    fi
}

# Check IPv6 support and modify nginx configs if needed
if ! check_ipv6_support && [ "$PROXMA_FORCE_IPV6" != "true" ]; then
    echo "[$(date)] IPv6 not supported, disabling IPv6 listeners in nginx configs"
    
    # Disable IPv6 in default.conf
    if [ -f /etc/nginx/conf.d/default.conf ]; then
        sed -i 's/^\(\s*listen \[::\].*\)/    # \1  # IPv6 disabled - not supported/' /etc/nginx/conf.d/default.conf
    fi
    
    # Disable IPv6 in any existing domain configs
    if [ -d /var/proxma/nginx/conf.d ]; then
        find /var/proxma/nginx/conf.d -name "*.conf" -type f -exec \
            sed -i 's/^\(\s*listen \[::\].*\)/    # \1  # IPv6 disabled - not supported/' {} \;
    fi
    
    echo "[$(date)] IPv6 listeners disabled (use PROXMA_FORCE_IPV6=true to override)"
else
    if [ "$PROXMA_FORCE_IPV6" = "true" ]; then
        echo "[$(date)] IPv6 forced via PROXMA_FORCE_IPV6 environment variable"
    else
        echo "[$(date)] IPv6 supported and enabled"
    fi
fi

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
# Start Nginx as a daemon
nginx
# Wait for Nginx to start
sleep 5
echo "[$(date)] Nginx started"

# Start the main application
exec /usr/local/bin/proxma