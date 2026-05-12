#!/bin/sh

if [[ -z ${1+x} ]] || [[ $1 == v* ]]; then
    echo "Usage: $0 <bytes to generate to stdout>" 1>&2
    exit 1;
fi

head -c "$1" < /dev/urandom