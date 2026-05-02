# Use official Rust image (ADR-040 specifies 1.82-slim)
FROM rust:1.82-slim

# Install dependencies required by Kani
RUN apt-get update && apt-get install -y \
    python3 \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user for executing proofs securely
RUN groupadd -g 1000 kani && \
    useradd -r -u 1000 -g kani -m -s /bin/bash kani

USER kani
WORKDIR /home/kani

# Install cargo-kani
RUN cargo install --locked kani-verifier && \
    cargo kani setup

# The sandbox mounting point
WORKDIR /work

# Set entrypoint to run Kani proofs
ENTRYPOINT ["cargo", "kani"]
