# Build stage
FROM rust:1.83-bookworm AS builder

WORKDIR /app

# Copy manifests first for better layer caching
COPY Cargo.toml ./

# Create dummy source to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source
COPY src ./src

# Build the application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/wrldbldr-engine /app/wrldbldr-engine

# Create non-root user
RUN useradd -m -u 1000 wrldbldr
USER wrldbldr

EXPOSE 3000

CMD ["/app/wrldbldr-engine"]
