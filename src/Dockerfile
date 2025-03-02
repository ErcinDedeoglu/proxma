# ----------- 1. Builder Stage -----------
    FROM --platform=$BUILDPLATFORM rust:slim-bullseye AS builder

    # Install build dependencies
    RUN apt-get update && apt-get install -y build-essential libssl-dev pkg-config
    
    # Set working directory
    WORKDIR /app
    
    # Copy all files
    COPY . .
    
    # Build the application for the target platform
    ARG TARGETPLATFORM
    RUN case "$TARGETPLATFORM" in \
          "linux/amd64") \
            rustup target add x86_64-unknown-linux-musl && \
            apt-get install -y musl-tools && \
            cargo build --release --target x86_64-unknown-linux-musl && \
            cp /app/target/x86_64-unknown-linux-musl/release/proxma /app/proxma \
            ;; \
          "linux/arm64") \
            rustup target add aarch64-unknown-linux-musl && \
            apt-get install -y musl-tools && \
            cargo build --release --target aarch64-unknown-linux-musl && \
            cp /app/target/aarch64-unknown-linux-musl/release/proxma /app/proxma \
            ;; \
          *) \
            echo "Unsupported platform: $TARGETPLATFORM" && exit 1 \
            ;; \
        esac
    
    # ----------- 2. Final Runtime Stage -----------
    FROM --platform=$TARGETPLATFORM alpine:latest
    
    # Copy binary from builder stage (now in a common location)
    COPY --from=builder /app/proxma /usr/local/bin/proxma
    
    # Make sure it's executable
    RUN chmod +x /usr/local/bin/proxma
    
    # Set binary entrypoint
    ENTRYPOINT ["/usr/local/bin/proxma"]