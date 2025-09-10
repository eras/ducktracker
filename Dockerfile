# Dockerfile for building an image with /usr/local/bin/ducktracker
# Use the switch release_switch=--release to build in release mode (much smaller binary)

# Setup node and modules
FROM node:iron-trixie-slim as node
WORKDIR /work
COPY frontend/ frontend
RUN cd frontend; npm ci

# Setup rust, cache Rust packages
FROM rust:trixie as rust
ARG release_switch
RUN apt-get update && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-build-deps
WORKDIR /work
# Trick to optimize Docker cache usage
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo 'fn main() { println!("dummy main"); }' > src/main.rs
RUN cargo build ${release_switch}
RUN rm -rf src target/*/ducktracker
# End of trick

# Generate frontend/src/bindings
COPY src/ src
COPY scripts/ scripts
RUN scripts/export-models-types.sh ${release_switch}

# Generate assets with vite
FROM node
RUN cd frontend; npm run build

# Copy assets from node to rust, so they can be embedded inside the binary, and build binary
FROM rust as rust-phase2
COPY src/ src
COPY --from=node /work/frontend/dist/ frontend/dist
RUN cargo build ${release_switch}
RUN cp target/*/ducktracker ducktracker; strip ducktracker

# Final image
FROM debian:trixie-slim
WORKDIR /data
COPY --from=rust-phase2 /work/ducktracker /usr/local/bin/ducktracker
ENTRYPOINT ["/usr/local/bin/ducktracker"]
