# ============= STAGE 1: Golang Builder ============= 
FROM golang:1.24 AS builder
# Set working directory explicitly
WORKDIR /app
# Copy Go module files explicitly (for caching)
COPY proxy-manager/go.mod proxy-manager/go.sum ./
RUN go mod download
# Explicitly copy all Go source files
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

# Explicitly create required directories
RUN mkdir -p \
    /var/run/nginx \
    /etc/nginx/conf.d \
    /run/supervisor \
    /var/www/certbot \
    /var/www/default_page \
    /var/log/proxy-manager

# Explicitly copy default landing page
COPY default_page/index.html /var/www/default_page/index.html

# Explicitly copy HTTP-only default.conf for landing page
COPY default.conf /etc/nginx/conf.d/default.conf

# Copy NGINX global config explicitly
COPY nginx.conf /etc/nginx/nginx.conf

# Copy Supervisor config explicitly
COPY supervisor.conf /etc/supervisor/conf.d/supervisor.conf

# Explicitly copy compiled Go binary  
COPY --from=builder /app/proxy-manager /usr/local/bin/proxy-manager
RUN chmod +x /usr/local/bin/proxy-manager

# Explicitly copy entrypoint and scripts explicitly
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

COPY proxy-manager-retry.sh /usr/local/bin/proxy-manager-retry.sh
RUN chmod +x /usr/local/bin/proxy-manager-retry.sh

# Explicitly expose ports
EXPOSE 80 443

# Explicitly set entrypoint
ENTRYPOINT ["/entrypoint.sh"]