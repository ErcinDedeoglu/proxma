# ============= STAGE 1: Golang Builder ============= 
FROM golang:1.24 AS builder
WORKDIR /app
COPY proxy-manager/go.mod proxy-manager/go.sum ./
RUN go mod download
COPY proxy-manager/*.go ./
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o proxy-manager

# ============= STAGE 2: Final Runtime Container ============= 
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    nginx \
    supervisor \
    curl \
    apt-transport-https \
    ca-certificates \
    gnupg \
    lsb-release \
    certbot \
    python3-certbot-nginx \
    cron \
  && rm -rf /var/lib/apt/lists/* \
  # Create nginx user
  && adduser --system --no-create-home --shell /bin/false --group --disabled-login nginx \
  # Create required directories
  && mkdir -p \
    /var/www/default_page \
    /var/www/certbot \
    /var/www/html \
    /etc/nginx/conf.d \
    /etc/nginx/default_available \
    /etc/certificates \
    /var/log/nginx \
    /var/log/proxy-manager \
    /var/run/nginx \
    /var/cache/nginx \
    /var/lib/nginx \
    /run/supervisor \
    /var/spool/cron/crontabs \
  # Set permissions
  && touch /var/run/nginx.pid \
  && touch /etc/crontab \
  && chmod 600 /etc/crontab \
  && chmod -R 600 /var/spool/cron/crontabs \
  && chown -R nginx:nginx \
    /var/www/default_page \
    /var/www/certbot \
    /var/www/html \
    /var/log/nginx \
    /var/run/nginx \
    /var/cache/nginx \
    /var/lib/nginx \
    /var/run/nginx.pid

# Install Docker CLI
RUN curl -fsSL https://download.docker.com/linux/debian/gpg | \
    gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] \
    https://download.docker.com/linux/debian $(lsb_release -cs) stable" \
    > /etc/apt/sources.list.d/docker.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends docker-ce-cli \
    && rm -rf /var/lib/apt/lists/*

# Copy configurations and binaries
COPY nginx.conf /etc/nginx/nginx.conf
COPY supervisor.conf /etc/supervisor/conf.d/supervisor.conf
COPY --from=builder /app/proxy-manager /usr/local/bin/proxy-manager
COPY proxy-manager-retry.sh /usr/local/bin/proxy-manager-retry.sh
COPY entrypoint.sh /entrypoint.sh

# Set executable permissions
RUN chmod +x \
    /usr/local/bin/proxy-manager \
    /usr/local/bin/proxy-manager-retry.sh \
    /entrypoint.sh

# Expose ports
EXPOSE 80 443

# Set entrypoint
ENTRYPOINT ["/entrypoint.sh"]