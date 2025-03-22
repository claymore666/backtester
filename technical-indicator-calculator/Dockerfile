FROM rust:1.85.1-slim as builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    wget \
    dpkg \
    && rm -rf /var/lib/apt/lists/*

# Install TA-Lib from GitHub release
WORKDIR /tmp
RUN wget https://github.com/ta-lib/ta-lib/releases/download/v0.6.4/ta-lib_0.6.4_amd64.deb && \
    dpkg -i ta-lib_0.6.4_amd64.deb && \
    rm ta-lib_0.6.4_amd64.deb

# Create a new empty project for dependency compilation only
WORKDIR /app
RUN cargo new --bin technical-indicator-calculator
WORKDIR /app/technical-indicator-calculator

# Copy only files needed for dependency resolution
COPY Cargo.toml Cargo.lock build.rs ./

# Create dummy main and modules to satisfy the Rust compiler
# We'll create empty files for all the module paths our actual code uses
RUN mkdir -p src/cache src/database src/indicators/patterns src/processor src/utils && \
    touch src/main.rs src/talib_bindings.rs && \
    touch src/cache/mod.rs src/cache/redis.rs && \
    touch src/database/mod.rs src/database/models.rs src/database/postgres.rs src/database/schema.rs && \
    touch src/indicators/mod.rs src/indicators/calculator.rs src/indicators/oscillators.rs && \
    touch src/indicators/overlaps.rs src/indicators/volatility.rs src/indicators/volume.rs && \
    touch src/indicators/patterns/mod.rs src/indicators/patterns/recognizer.rs && \
    touch src/indicators/patterns/single_candle.rs src/indicators/patterns/double_candle.rs && \
    touch src/indicators/patterns/triple_candle.rs src/indicators/patterns/utils.rs && \
    touch src/processor/mod.rs src/processor/job.rs src/processor/worker.rs && \
    touch src/utils/utils.rs

# Build dependencies only
RUN RUSTFLAGS="-C link-arg=-lta-lib" cargo build --release

# Now that dependencies are built, remove the dummy files and copy real source
RUN find src -type f -delete

# Copy the actual source code
COPY src/ src/

# Build the application
RUN RUSTFLAGS="-C link-arg=-lta-lib" cargo build --release

# Create runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    libssl3 \
    ca-certificates \
    wget \
    dpkg \
    && rm -rf /var/lib/apt/lists/*

# Install TA-Lib from GitHub release
WORKDIR /tmp
RUN wget https://github.com/ta-lib/ta-lib/releases/download/v0.6.4/ta-lib_0.6.4_amd64.deb && \
    dpkg -i ta-lib_0.6.4_amd64.deb && \
    rm ta-lib_0.6.4_amd64.deb

# Copy the built binary
COPY --from=builder /app/technical-indicator-calculator/target/release/technical-indicator-calculator /usr/local/bin/

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/technical-indicator-calculator"]
