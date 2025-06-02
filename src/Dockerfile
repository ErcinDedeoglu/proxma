# Stage 1: Chef setup
FROM rust:alpine AS chef
RUN apk add --no-cache musl-dev build-base openssl-dev openssl-libs-static pkgconfig curl && \
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
    RUST_BACKTRACE=1 cargo build -v --release --target $RUST_TARGET && \
    cp target/$RUST_TARGET/release/proxma /proxma

# Final stage
FROM alpine:latest
RUN apk add --no-cache nginx tini certbot openssl curl bind-tools certbot-dns-cloudflare dos2unix dcron logrotate fail2ban

RUN mkdir -p /run/nginx /var/www/html/.well-known/acme-challenge /etc/nginx/ssl /var/proxma/ssl /var/proxma/logs /etc/nginx/cache-zones.d /var/proxma/htpasswd /var/proxma/nginx/conf.d /var/proxma/nginx/includes /var/proxma/nginx/logs && \
    chown -R nginx:nginx /var/www/html /run/nginx /etc/nginx/cache-zones.d /var/proxma/htpasswd /var/proxma/nginx \
    && openssl req -x509 -nodes -days 3650 -newkey rsa:2048 \
    -keyout /var/proxma/ssl/default.privkey.key.pem \
    -out /var/proxma/ssl/default.fullchain.crt.pem \
    -subj "/C=US/ST=California/L=San Francisco/O=Dublok/OU=Proxma/CN=dublok.com/emailAddress=dublokcom@gmail.com" \
    && chmod 644 /var/proxma/ssl/default.fullchain.crt.pem \
    && chmod 600 /var/proxma/ssl/default.privkey.key.pem

# Copy all scripts
COPY src/docker_scripts/fetch_cloudflare_ips.sh /usr/local/bin/
COPY src/docker_scripts/update_cloudflare_ips.sh /usr/local/bin/
COPY src/docker_scripts/startup.sh /usr/local/bin/

# Fail2ban configuration with comprehensive filter setup
RUN mkdir -p /etc/fail2ban/action.d \
    && mkdir -p /etc/fail2ban/filter.d \
    && mkdir -p /etc/nginx/conf.d/banned \
    && mkdir -p /var/run/fail2ban

# Copy all filter configurations using wildcard pattern
COPY src/fail2ban/filter.d/*.conf /etc/fail2ban/filter.d/
COPY src/fail2ban/action.d/nginx-deny.conf /etc/fail2ban/action.d/
COPY src/fail2ban/jail.local /etc/fail2ban/

# Set permissions and create required files
RUN chmod 644 /etc/fail2ban/jail.local \
    && chmod -R 644 /etc/fail2ban/filter.d/* \
    && chmod 644 /etc/fail2ban/action.d/* \
    && mkdir -p /etc/nginx/conf.d/banned \
    && touch /etc/nginx/conf.d/banned/banned_ips.conf \
    && chmod 644 /etc/nginx/conf.d/banned/banned_ips.conf

# Copy logrotate configuration
COPY src/docker_scripts/proxma-logrotate.conf /etc/logrotate.d/
COPY src/docker_scripts/run_logrotate.sh /usr/local/bin/

# Make the script executable and convert line endings if needed
RUN dos2unix /usr/local/bin/run_logrotate.sh && \
    chmod +x /usr/local/bin/run_logrotate.sh

# Make scripts executable and convert line endings if needed
RUN dos2unix /usr/local/bin/fetch_cloudflare_ips.sh /usr/local/bin/update_cloudflare_ips.sh /usr/local/bin/startup.sh && \
    chmod +x /usr/local/bin/fetch_cloudflare_ips.sh /usr/local/bin/update_cloudflare_ips.sh /usr/local/bin/startup.sh && \
    /usr/local/bin/fetch_cloudflare_ips.sh

# Set up cron job to run every night at 2:00 AM
RUN echo '0 2 * * * /usr/local/bin/update_cloudflare_ips.sh' > /etc/crontabs/root && \
    echo '0 0 * * * /usr/local/bin/run_logrotate.sh' >> /etc/crontabs/root && \
    chmod 0600 /etc/crontabs/root

COPY src/webserver/nginx/nginx.conf /etc/nginx/nginx.conf
COPY src/webserver/nginx/conf.d/default.conf /etc/nginx/conf.d/default.conf

# Modify the default.conf to include the Cloudflare IPs configuration
RUN sed -i '/server {/i include /etc/nginx/conf.d/includes/cloudflare_ips.conf;' /etc/nginx/conf.d/default.conf

COPY src/webserver/website /var/www/html
RUN chmod -R 755 /var/www/html
RUN echo "proxma" > /var/www/html/.well-known/acme-challenge/proxma.proxma \
    && chmod 644 /var/www/html/.well-known/acme-challenge/proxma.proxma
COPY --from=builder /proxma /usr/local/bin/proxma
HEALTHCHECK --interval=30s --timeout=3s CMD nginx -t

ENTRYPOINT ["/sbin/tini", "--"]
CMD ["/usr/local/bin/startup.sh"]