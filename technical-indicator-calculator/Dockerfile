FROM debian:bookworm-slim

# Install runtime dependencies only
RUN apt-get update && \
    apt-get install -y \
    libssl3 \
    ca-certificates \
    wget \
    dpkg \
    vim \
    && rm -rf /var/lib/apt/lists/*

# Install TA-Lib
WORKDIR /tmp
RUN wget https://github.com/ta-lib/ta-lib/releases/download/v0.6.4/ta-lib_0.6.4_amd64.deb && \
    dpkg -i ta-lib_0.6.4_amd64.deb && \
    rm ta-lib_0.6.4_amd64.deb

# Create app directory
WORKDIR /app

# Copy the pre-built binary
COPY target/release/technical-indicator-calculator /usr/local/bin/

# Copy source for reference (optional)
COPY src/ /app/src/
COPY Cargo.toml Cargo.lock /app/

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/technical-indicator-calculator"]
