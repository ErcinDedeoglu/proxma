# ----------- 1. Builder Stage -----------
    FROM rust:slim-bullseye AS builder

    # Install build dependencies
    RUN apt-get update && apt-get install -y build-essential libssl-dev pkg-config
    
    # Set working directory
    WORKDIR /app
    
    # Copy all files
    COPY . .
    
    # Build with MUSL for static linking (fully self-contained binary)
    RUN rustup target add x86_64-unknown-linux-musl
    RUN apt-get install -y musl-tools
    RUN cargo build --release --target x86_64-unknown-linux-musl
    
    # ----------- 2. Final Runtime Stage -----------
    FROM alpine:latest
    
    # Copy statically linked binary from builder stage
    COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/proxma /usr/local/bin/proxma
    
    # Make sure it's executable
    RUN chmod +x /usr/local/bin/proxma
    
    # Set binary entrypoint
    ENTRYPOINT ["/usr/local/bin/proxma"]