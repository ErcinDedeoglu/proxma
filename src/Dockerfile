FROM rust:alpine AS chef
RUN apk add --no-cache musl-dev build-base && \
    rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl && \
    cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS cacher
COPY --from=planner /app/recipe.json recipe.json
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
        linux/amd64) export RUST_TARGET=x86_64-unknown-linux-musl ;; \
        linux/arm64) export RUST_TARGET=aarch64-unknown-linux-musl ;; \
        *) echo "Unsupported $TARGETPLATFORM" && exit 1 ;; \
    esac && \
    cargo chef cook --release --target $RUST_TARGET --recipe-path recipe.json

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

FROM alpine:latest
RUN apk add --no-cache nginx

# Copy Nginx configuration and website
COPY src/nginx.conf /etc/nginx/http.d/default.conf
COPY src/website /var/www/html

# Copy Rust binary
COPY --from=builder /proxma /usr/local/bin/proxma

# Create nginx runtime directory and start services
RUN mkdir -p /run/nginx
CMD (nginx -g "daemon off;" &) && exec /usr/local/bin/proxma