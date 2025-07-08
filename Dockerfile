# Use the official Rust image as a builder
FROM rust:1.75-slim as builder

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

# Build the single validator binary
RUN cd bin && cargo build --release --bin single-validator

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

# Copy the binary from builder
COPY --from=builder /app/bin/target/release/single-validator /usr/local/bin/

# Create data directory
RUN mkdir -p /app/data && chown -R mysticeti:mysticeti /app

# Switch to non-root user
USER mysticeti

# Expose default ports
EXPOSE 26657 26670

# Set default command
ENTRYPOINT ["single-validator"]
CMD ["--help"] 