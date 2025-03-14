#!/bin/sh
# Define the output file
OUTPUT_FILE="/etc/nginx/conf.d/includes/cloudflare_ips.conf"

# Create directory if it doesn't exist
mkdir -p /etc/nginx/conf.d/includes

# Fetch the latest Cloudflare IP ranges and format them for Nginx
{
    echo "# Cloudflare IP Ranges - Generated on $(date)"
    echo ""
    echo "# IPv4"
    curl -s https://www.cloudflare.com/ips-v4/ | while read -r ip; do
        echo "set_real_ip_from $ip;"
    done
    echo ""
    echo "# IPv6"
    curl -s https://www.cloudflare.com/ips-v6/ | while read -r ip; do
        echo "set_real_ip_from $ip;"
    done
    echo ""
    echo "real_ip_header CF-Connecting-IP;"
    echo "real_ip_recursive on;"
} > "$OUTPUT_FILE"

echo "Cloudflare IP configuration generated at $OUTPUT_FILE"