FROM rust:1.81-slim as builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install TA-Lib
WORKDIR /tmp
RUN apt-get update && apt-get install -y wget make
RUN wget http://prdownloads.sourceforge.net/ta-lib/ta-lib-0.4.0-src.tar.gz && \
    tar -xzf ta-lib-0.4.0-src.tar.gz && \
    cd ta-lib/ && \
    ./configure --prefix=/usr && \
    make && \
    make install

# Create a new empty project
WORKDIR /app
RUN cargo new --bin technical-indicator-calculator
WORKDIR /app/technical-indicator-calculator

# Copy manifests
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock

# Build dependencies only (this is a trick to cache dependencies)
RUN cargo build --release
RUN rm src/*.rs

# Copy source code
COPY ./src ./src

# Build the application
RUN touch src/main.rs
RUN cargo build --release

# Create runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install TA-Lib runtime
COPY --from=builder /usr/lib/libta_lib* /usr/lib/
RUN ldconfig

# Copy the built binary
COPY --from=builder /app/technical-indicator-calculator/target/release/technical-indicator-calculator /usr/local/bin/

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/technical-indicator-calculator"]
