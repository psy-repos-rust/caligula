#!/usr/bin/env bash

# Reads the pid stored in ./pid, and attaches GDB to the PID inside the VM.

set -euxo pipefail

pid="$(cat pid)"

exec rust-gdb \
    -ex "target extended-remote devvm_gdb.sock" \
    -ex "attach $(cat pid)" \
    "$@"

