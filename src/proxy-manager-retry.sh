#!/bin/bash
echo "♻️ Starting hourly SSL certificate retry task at $(date)..."

/usr/local/bin/proxy-manager & sleep 5 && kill $!

echo "♻️ SSL certificate retry task completed at $(date)."