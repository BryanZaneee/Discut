#!/bin/sh
# Keep local native builds within a laptop-sized memory/disk budget.
set -eu
cd "$(dirname "$0")/.."
export PATH="$PWD/target/build-tools/bin:$PATH"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_PROFILE_RELEASE_DEBUG=0
if [ "$#" -eq 0 ]; then
  set -- xtask package
fi
exec cargo "$@"
