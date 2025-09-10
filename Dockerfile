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
COPY Cargo.toml Cargo.lock build.rs ./
RUN mkdir -p src && echo 'fn main() { println!("dummy main"); }' > src/main.rs
# End of trick

# Generate frontend/src/bindings
COPY src/ src
COPY scripts/ scripts
RUN scripts/export-models-types.sh ${release_switch} && rm -rf src target

# Generate assets with vite
FROM node
RUN cd frontend; npm run build

FROM rust:trixie as rust-phase2
RUN apt-get update && apt install -y git && rm -rf /var/lib/apt/lists/*
# Use another directory, so Docker won't be confused by overwriting files
WORKDIR /work
COPY Cargo.toml Cargo.lock build.rs .
COPY src/ src
# Tried to trick cargo not to rebuild everything, but it didn't stick
# This whole tomfoolery is because Docker gets confused by overwriting files (such as the dummy main.rs before).
# ..but it doesn't help.
#RUN cp -r ../target target
# Copy assets from node to rust, so they can be embedded inside the binary, and build binary
COPY --from=node /work/frontend/dist/ /work/frontend/dist
COPY .git/ .git
RUN ls -Rl frontend/dist
RUN git describe
RUN pwd
RUN ls -la
RUN cargo build ${release_switch} && cp target/*/ducktracker /work/ducktracker && /work/ducktracker --version | grep 'ducktracker' && rm -rf target

# Final image
FROM debian:trixie-slim
WORKDIR /data
COPY --from=rust-phase2 /work/ducktracker /usr/local/bin/ducktracker
ENTRYPOINT ["/usr/local/bin/ducktracker"]
