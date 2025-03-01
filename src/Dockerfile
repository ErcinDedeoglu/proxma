# ============= STAGE 1: Golang Builder ============= 
FROM golang:1.24 AS builder
# Set working directory explicitly
WORKDIR /app
# Copy Go module files explicitly (for caching clearly)
COPY proxy-manager/go.mod proxy-manager/go.sum ./
RUN go mod download
# Explicitly copy all Go source files clearly
COPY proxy-manager/*.go ./
# Build the proxy-manager binary explicitly
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o proxy-manager

# ============= STAGE 2: Final Runtime Container ============= 
FROM debian:bullseye-slim
# Install runtime dependencies explicitly
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
  && rm -rf /var/lib/apt/lists/*

# Install Docker CLI explicitly
RUN curl -fsSL https://download.docker.com/linux/debian/gpg | \
    gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] \
    https://download.docker.com/linux/debian $(lsb_release -cs) stable" \
    > /etc/apt/sources.list.d/docker.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends docker-ce-cli \
    && rm -rf /var/lib/apt/lists/*

# Explicitly create required directories explicitly clearly including cron and nginx configuration locations
RUN mkdir -p \
    /var/run/nginx \
    /etc/nginx/conf.d \
    /etc/nginx/default_available \
    /run/supervisor \
    /var/www/certbot \
    /var/www/default_page \
    /var/log/proxy-manager \
    /var/spool/cron/crontabs \
  && touch /etc/crontab \
  && chmod 600 /etc/crontab \
  && chmod -R 600 /var/spool/cron/crontabs

# Explicitly copy default landing page HTML explicitly clearly
COPY default_page/index.html /var/www/default_page/index.html

# Explicitly copy preserved (default) nginx config explicitly clearly
COPY default.conf /etc/nginx/default_available/default.conf

# Explicitly copy global nginx config explicitly clearly
COPY nginx.conf /etc/nginx/nginx.conf

# Explicitly copy supervisor config explicitly clearly
COPY supervisor.conf /etc/supervisor/conf.d/supervisor.conf

# Explicitly copy compiled Golang binary explicitly clearly from the builder stage  
COPY --from=builder /app/proxy-manager /usr/local/bin/proxy-manager
RUN chmod +x /usr/local/bin/proxy-manager

# Explicitly Entrypoint and Retry Helper explicitly clearly
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh
COPY proxy-manager-retry.sh /usr/local/bin/proxy-manager-retry.sh
RUN chmod +x /usr/local/bin/proxy-manager-retry.sh

# Expose HTTP and HTTPS clearly explicitly
EXPOSE 80 443

# Explicitly define entrypoint explicitly explicitly
ENTRYPOINT ["/entrypoint.sh"]