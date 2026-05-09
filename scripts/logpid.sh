#!/bin/sh
# Logs the current PID to stderr and a file, and execs the commands given.
# Useful when combined with gdbvm.sh.
echo "$$" | tee pid 1>&2
exec "$@"