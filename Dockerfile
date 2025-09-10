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

FROM rust as rust-phase2
# Use another directory, so Docker won't be confused by overwriting files
RUN mkdir build
WORKDIR /work
COPY Cargo.toml Cargo.lock build
COPY src/ build/src
WORKDIR /work/build
# Tried to trick cargo not to rebuild everything, but it didn't stick
# This whole tomfoolery is because Docker gets confused by overwriting files (such as the dummy main.rs before).
# ..but it doesn't help.
RUN cp -r ../target target
# Copy assets from node to rust, so they can be embedded inside the binary, and build binary
COPY --from=node /work/frontend/dist/ frontend/dist
RUN cargo build ${release_switch}
RUN cp target/*/ducktracker /work/ducktracker; strip /work/ducktracker
# Sanity test
RUN /work/ducktracker --help | grep 'Usage'

# Final image
FROM debian:trixie-slim
WORKDIR /data
COPY --from=rust-phase2 /work/ducktracker /usr/local/bin/ducktracker
ENTRYPOINT ["/usr/local/bin/ducktracker"]
