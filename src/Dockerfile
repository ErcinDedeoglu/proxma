# Stage 1: Golang builder
FROM golang:1.24 AS builder
WORKDIR /app
COPY proxy-manager/go.mod proxy-manager/go.sum ./
RUN go mod download
COPY proxy-manager/*.go ./
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -o proxy-manager main.go

# Stage 2: Final combined container
FROM debian:bullseye-slim

# Install dependencies
RUN apt-get update && apt-get install -y \
    nginx \
    supervisor \
    curl \
    apt-transport-https \
    ca-certificates \
    gnupg \
    lsb-release && \
    rm -rf /var/lib/apt/lists/*

# Add Docker's official GPG key
RUN curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg

# Add Docker repository
RUN echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/debian $(lsb_release -cs) stable" > /etc/apt/sources.list.d/docker.list

# Install Docker CLI
RUN apt-get update && apt-get install -y docker-ce-cli && rm -rf /var/lib/apt/lists/*

# Create necessary directories
RUN mkdir -p /var/run/nginx && \
    mkdir -p /etc/nginx/conf.d && \
    mkdir -p /run/supervisor && \
    mkdir -p /var/log

# Copy Nginx basic config
COPY nginx.conf /etc/nginx/nginx.conf

# Copy proxy-manager binary
COPY --from=builder /app/proxy-manager /usr/local/bin/proxy-manager

# Ensure the binary is executable
RUN chmod +x /usr/local/bin/proxy-manager

# Copy supervisord configuration
COPY supervisor.conf /etc/supervisor/conf.d/supervisor.conf

# Expose port 80 (HTTP)
EXPOSE 80

# Start supervisord to run both nginx and proxy-manager
CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/supervisor.conf"]