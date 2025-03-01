# ============= STAGE 1: Golang Builder ============= 
FROM golang:1.24 AS builder

# Set working directory clearly
WORKDIR /app

# First, copy module files explicitly for better caching (if you have go.mod)
COPY proxy-manager/go.mod proxy-manager/go.sum ./
RUN go mod download

# Copy all your Go source files clearly
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
    /var/log/proxy-manager

# Copy NGINX server configuration explicitly
COPY nginx.conf /etc/nginx/nginx.conf
COPY supervisor.conf /etc/supervisor/conf.d/supervisor.conf

# Explicitly copy compiled Go binary from builder stage  
COPY --from=builder /app/proxy-manager /usr/local/bin/proxy-manager
RUN chmod +x /usr/local/bin/proxy-manager

# Explicitly copy improved entrypoint script and cronjob retry helper clearly
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

COPY proxy-manager-retry.sh /usr/local/bin/proxy-manager-retry.sh
RUN chmod +x /usr/local/bin/proxy-manager-retry.sh

# Expose ports explicitly and clearly
EXPOSE 80 443

# Entrypoint clearly defined 
ENTRYPOINT ["/entrypoint.sh"]