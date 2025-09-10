#!/bin/sh
TAG="$1"
set -x -e
scripts/export-models-types.sh --release
(cd frontend && npm install && npm run build)
cargo build --release
cp target/release/ducktracker .
docker build -f Dockerfile.local -t ${TAG:-ducktracker} .
