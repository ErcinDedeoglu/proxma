# Stage 1: Chef setup
FROM rust:alpine AS chef
RUN apk add --no-cache musl-dev build-base && \
    rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl && \
    cargo install cargo-chef
WORKDIR /app

# Stage 2: Recipe planner
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Dependency caching
FROM chef AS cacher
COPY --from=planner /app/recipe.json recipe.json
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
        linux/amd64) export RUST_TARGET=x86_64-unknown-linux-musl ;; \
        linux/arm64) export RUST_TARGET=aarch64-unknown-linux-musl ;; \
        *) echo "Unsupported $TARGETPLATFORM" && exit 1 ;; \
    esac && \
    cargo chef cook --release --target $RUST_TARGET --recipe-path recipe.json

# Stage 4: Builder
FROM chef AS builder
COPY . .
COPY --from=cacher /app/target target
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
        linux/amd64) export RUST_TARGET=x86_64-unknown-linux-musl ;; \
        linux/arm64) export RUST_TARGET=aarch64-unknown-linux-musl ;; \
        *) echo "Unsupported $TARGETPLATFORM" && exit 1 ;; \
    esac && \
    cargo build --release --target $RUST_TARGET && \
    cp target/$RUST_TARGET/release/proxma /proxma

# Final stage
FROM alpine:latest

# Add OpenSSL to the packages
RUN apk add --no-cache nginx tini certbot openssl

# Create directories and set permissions
RUN mkdir -p /run/nginx /var/www/html/.well-known/acme-challenge /etc/nginx/ssl /var/proxma/ssl && \
    chown -R nginx:nginx /var/www/html /run/nginx

# Generate self-signed certificate
RUN openssl req -x509 -nodes -days 3650 -newkey rsa:2048 \
    -keyout /var/proxma/ssl/default.key \
    -out /var/proxma/ssl/default.crt \
    -subj "/C=US/ST=California/L=San Francisco/O=Dublok/OU=Proxma/CN=dublok.com/emailAddress=dublokcom@gmail.com" && \
    chmod 644 /var/proxma/ssl/default.crt && \
    chmod 600 /var/proxma/ssl/default.key
    
# Copy files
COPY src/webserver/nginx/nginx.conf /etc/nginx/nginx.conf
COPY src/webserver/nginx/conf.d/default.conf /etc/nginx/conf.d/default.conf
COPY src/webserver/website /var/www/html

# Ensure proper permissions
RUN chmod -R 755 /var/www/html

# Copy Rust binary
COPY --from=builder /proxma /usr/local/bin/proxma

# Health check
HEALTHCHECK --interval=30s --timeout=3s CMD nginx -t

# Use tini as the entrypoint
ENTRYPOINT ["/sbin/tini", "--"]

# Start both nginx and proxma
CMD ["sh", "-c", "nginx -g \"daemon off;\" & exec /usr/local/bin/proxma"]
