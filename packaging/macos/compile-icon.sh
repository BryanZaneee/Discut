#!/bin/sh
# The bundled ICNS works on supported macOS versions without a full Xcode installation.
set -eu
resources="$1"
mkdir -p "$resources"
cp packaging/macos/Serein.icns "$resources/Serein.icns"
