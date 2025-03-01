# Stage 1: Golang builder
FROM golang:1.24 AS builder

WORKDIR /app
COPY proxy-manager/go.mod proxy-manager/go.sum ./
RUN go mod download

COPY proxy-manager/*.go ./
RUN go build -o proxy-manager main.go

# Stage 2: Final combined container
FROM alpine:latest

RUN apk add --no-cache nginx supervisor curl docker-cli

# Create necessary directories
RUN mkdir -p /var/run/nginx && \
    mkdir -p /etc/nginx/conf.d && \
    mkdir -p /run/supervisor

# Copy Nginx basic config
COPY nginx.conf /etc/nginx/nginx.conf

# Copy proxy-manager binary
COPY --from=builder /app/proxy-manager /usr/local/bin/proxy-manager

# Copy supervisord configuration
COPY supervisor.conf /etc/supervisor/conf.d/supervisor.conf

# Expose port 80 (HTTP)
EXPOSE 80

# Start supervisord to run both nginx and proxy-manager
CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/supervisor.conf"]