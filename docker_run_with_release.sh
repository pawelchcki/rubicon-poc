#!/bin/bash -xe
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

docker run \
    -v "$SCRIPT_DIR/target/x86_64-unknown-linux-none/release/librubicon_poc.so:/librubicon_poc.so"\
    -v "$SCRIPT_DIR/ld.preload:/etc/ld.so.preload" \
    $@
