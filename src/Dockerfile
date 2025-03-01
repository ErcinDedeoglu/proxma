# Stage 1: Golang builder
FROM golang:1.24 AS builder
WORKDIR /app

# Copy go.mod/go.sum and download dependencies
COPY proxy-manager/go.mod proxy-manager/go.sum ./
RUN go mod download

# Now copy all Go files clearly from your actual correct path (src/proxy-manager)
COPY proxy-manager/*.go ./

# Build the binary by compiling all Go files (note the "./" at the end)
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o proxy-manager ./

# Stage 2: Final combined container
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    nginx supervisor curl apt-transport-https ca-certificates gnupg lsb-release certbot python3-certbot-nginx cron \
  && rm -rf /var/lib/apt/lists/*

# Install Docker CLI
RUN curl -fsSL https://download.docker.com/linux/debian/gpg \
  | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg \
  && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] \
  https://download.docker.com/linux/debian $(lsb_release -cs) stable" \
  > /etc/apt/sources.list.d/docker.list \
  && apt-get update && apt-get install -y docker-ce-cli \
  && rm -rf /var/lib/apt/lists/*

# Create directories
RUN mkdir -p /var/run/nginx \
    && mkdir -p /etc/nginx/conf.d \
    && mkdir -p /run/supervisor \
    && mkdir -p /var/log \
    && mkdir -p /var/www/certbot

# Copy nginx and supervisor configs
COPY nginx.conf /etc/nginx/nginx.conf
COPY supervisor.conf /etc/supervisor/conf.d/supervisor.conf

# Copy binary from builder stage
COPY --from=builder /app/proxy-manager /usr/local/bin/proxy-manager
RUN chmod +x /usr/local/bin/proxy-manager

# Add entrypoint script
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Ports
EXPOSE 80 443

# Entrypoint
ENTRYPOINT ["/entrypoint.sh"]