#!/bin/sh
# So this works out-of-scratch.. Egg vs chicken..
mkdir frontend/dist
touch frontend/dist/index.html
TS_RS_EXPORT_DIR=frontend/src/bindings cargo test "$@"
