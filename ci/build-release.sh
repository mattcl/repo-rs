#!/bin/sh
set -ex

if command -v apk > /dev/null; then
    apk add openssl \
        openssl-dev
else
    apt-get update && apt-get install -y \
        libssl-dev \
        pkg-config
fi

cd repo
cargo build --release
cd ../

# this is fragile: we're going to assume we always have a --version flag
VERSION=$("repo/target/release/$BIN_NAME" --version | cut -d " " -f 2)

# we need a way to reference the version
echo "$VERSION" > release/VERSION

echo "Packaging $VERSION for $TARGET"

mkdir dist
cp "repo/target/release/$BIN_NAME" dist/

cd dist
ARCHIVE="${BIN_NAME}-${VERSION}-${TARGET}.tar.gz"
tar czf "$ARCHIVE" "$BIN_NAME"
cd ../

mv "dist/$ARCHIVE" "release/$ARCHIVE"

# we need a way to reference the file name
echo "$ARCHIVE" > release/ARCHIVE_NAME
