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

# We need to work around a current limitaiton in `cargo msrv` where it can't
# handle verifying the version specified by a workspace.
# https://github.com/foresterre/cargo-msrv/issues/590
#
# So what we're going to do is determine the minimum version that's declared in
# the Cargo.toml at the root of the repository and explicitly constrain
# `cargo msrv` to that version using `--min` and `--max`

rust_version=$(grep 'rust-version' Cargo.toml | cut -d '"' -f 2)

if [ "$UPDATE_DEPS" -eq 1 ]; then
    cargo update
fi

# just a basic check
cargo msrv --min=${rust_version} --max=${rust_version}

# verify we can run tests at this version
cargo msrv --min=${rust_version} --max=${rust_version} -- cargo test --locked --all-targets
