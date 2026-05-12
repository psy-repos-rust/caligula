#!/bin/sh

set -eux

if [ $# -ne 2 ]; then
    echo "Usage: $0 <bytecount> <outfile>" 1>&2
    exit 1;
fi

outfile="$2"

head -c "$1" < /dev/urandom > "$outfile"
gzip -k "$outfile"
lz4 -k "$outfile"
xz -k "$outfile"
bzip2 -k "$outfile"
sha256sum "$outfile" "$outfile".* > SHA256SUMS