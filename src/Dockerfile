FROM rust:slim-bullseye AS builder

RUN apt-get update && apt-get install -y musl-tools build-essential && \
    rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl

WORKDIR /app
COPY . .

ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
        linux/amd64) export RUST_TARGET=x86_64-unknown-linux-musl ;; \
        linux/arm64) export RUST_TARGET=aarch64-unknown-linux-musl ;; \
        *) echo "Unsupported platform $TARGETPLATFORM" && exit 1 ;; \
    esac && \
    cargo build --release --target $RUST_TARGET && \
    cp target/$RUST_TARGET/release/proxma /proxma

FROM alpine:latest
COPY --from=builder /proxma /usr/local/bin/proxma
ENTRYPOINT ["/usr/local/bin/proxma"]