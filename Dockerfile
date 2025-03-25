FROM rust:1.85 as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && \
    apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo.toml and Cargo.lock
COPY Cargo.toml .
COPY Cargo.lock* .

# Create dummy src/main.rs to build dependencies
RUN mkdir -p src && \
    echo "fn main() {println!(\"Dependency build complete!\")}" > src/main.rs

# Build dependencies (cached if Cargo.toml is unchanged)
RUN cargo build --release

# Remove the dummy source file
RUN rm -rf src

# Copy the actual source code
COPY . .

# Build the application
RUN cargo build --release

# Final image
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    libssl3 \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/solana-trade-bot /app/solana-trade-bot

# Copy migrations folder
COPY --from=builder /app/migrations /app/migrations

# Run the binary
CMD ["/app/solana-trade-bot"]