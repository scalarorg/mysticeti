# Use the official Rust image as a builder
FROM rust:1.88-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libclang-dev \
    clang \
    make \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy the entire workspace
COPY . .

# Build all binaries from execute crate
RUN cd execute && cargo build --release --bin validator --bin single-node --bin network

# Build all binaries from orchestrator crate
# RUN cd orchestrator && cargo build --release --bin orchestrator --bin local-network --bin remote-network

# Create a minimal runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1000 mysticeti

# Set working directory
WORKDIR /app

# Copy all binaries from builder
COPY --from=builder /app/target/release/validator /usr/local/bin/
COPY --from=builder /app/target/release/single-node /usr/local/bin/
COPY --from=builder /app/target/release/network /usr/local/bin/
# COPY --from=builder /app/target/release/orchestrator /usr/local/bin/
# COPY --from=builder /app/target/release/local-network /usr/local/bin/
# COPY --from=builder /app/target/release/remote-network /usr/local/bin/

# Create data directory
RUN mkdir -p /app/data && chown -R mysticeti:mysticeti /app

# Switch to non-root user
USER mysticeti

# Expose default ports
EXPOSE 26657 26670

# Set default command
ENTRYPOINT ["validator"]
CMD ["--help"] 