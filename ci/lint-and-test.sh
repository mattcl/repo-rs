#!/bin/sh
set -ex

if command -v apk > /dev/null; then
    apk add openssl \
        openssl-dev \
        pkgconfig
else
    apt-get update && apt-get install -y \
        libssl-dev \
        pkg-config
fi

if [ "$LINT" -eq 1 ]; then
    # make sure we're formatted
    cargo fmt --check

    # fail on clippy warnings
    cargo clippy -- -Dwarnings
fi

# ensure we can build
cargo build --verbose ${EXTRA_CARGO_BUILD_FLAGS}

# ensure tests pass
cargo test --verbose ${EXTRA_CARGO_TEST_FLAGS}
