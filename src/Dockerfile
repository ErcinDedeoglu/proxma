# ============= STAGE 1: Golang Builder ============= 
FROM golang:1.24 AS builder
# Set working directory explicitly
WORKDIR /app
# Copy Go module dependency files explicitly (better for layer caching)
COPY proxy-manager/go.mod proxy-manager/go.sum ./
RUN go mod download
# Copy all your Go source files explicitly
COPY proxy-manager/*.go ./
# Clearly build the binary executable explicitly for Linux
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o proxy-manager

# ============= STAGE 2: Final Runtime Container ============= 
FROM debian:bullseye-slim

# Install runtime dependencies explicitly and robustly
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
    openssl \
  && rm -rf /var/lib/apt/lists/*

# Explicitly install Docker CLI
RUN curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] \
    https://download.docker.com/linux/debian $(lsb_release -cs) stable" \
      > /etc/apt/sources.list.d/docker.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends docker-ce-cli \
    && rm -rf /var/lib/apt/lists/*

# Clearly create required directories explicitly
RUN mkdir -p \
    /var/run/nginx \
    /etc/nginx/conf.d \
    /run/supervisor \
    /var/www/certbot \
    /var/www/default_page \
    /var/log/proxy-manager

# Generate a self-signed SSL certificate explicitly (for initial default HTTPS site)
RUN mkdir -p /etc/ssl/private && chmod 700 /etc/ssl/private \
    && openssl req -x509 -nodes -days 3650 -newkey rsa:4096 \
    -keyout /etc/ssl/private/nginx-selfsigned.key \
    -out /etc/ssl/certs/nginx-selfsigned.crt \
    -subj "/CN=proxma-local"

# Copy explicitly defined default landing page
COPY default_page/index.html /var/www/default_page/index.html

# Explicitly copy default.conf which serves the default landing page explicitly
COPY default.conf /etc/nginx/conf.d/default.conf

# Copy NGINX server-wide configuration explicitly
COPY nginx.conf /etc/nginx/nginx.conf

# Copy supervisord configuration explicitly
COPY supervisor.conf /etc/supervisor/conf.d/supervisor.conf

# Explicitly copy compiled Go binary from the builder stage  
COPY --from=builder /app/proxy-manager /usr/local/bin/proxy-manager
RUN chmod +x /usr/local/bin/proxy-manager

# Explicitly copy entrypoint script and cronjob retry helper explicitly
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
COPY proxy-manager-retry.sh /usr/local/bin/proxy-manager-retry.sh
RUN chmod +x /usr/local/bin/proxy-manager-retry.sh

# Explicitly expose ports clearly
EXPOSE 80 443

# Explicitly define entrypoint clearly
ENTRYPOINT ["/entrypoint.sh"]