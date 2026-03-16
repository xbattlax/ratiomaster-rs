#!/bin/bash
set -euo pipefail

APP_NAME="RatioMaster"
BUNDLE="${APP_NAME}.app"
BINARY="ratiomaster-gui"
VERSION="0.1.0"
IDENTITY="${CODESIGN_IDENTITY:-}"

# Paths
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
TARGET_DIR="${PROJECT_DIR}/target/release"
BUNDLE_DIR="${PROJECT_DIR}/target/${BUNDLE}"

echo "Building release binary..."
cargo build --release -p ratiomaster-gui

echo "Creating app bundle..."
rm -rf "${BUNDLE_DIR}"
mkdir -p "${BUNDLE_DIR}/Contents/MacOS"
mkdir -p "${BUNDLE_DIR}/Contents/Resources"

# Copy binary
cp "${TARGET_DIR}/${BINARY}" "${BUNDLE_DIR}/Contents/MacOS/"

# Copy Info.plist
cp "${PROJECT_DIR}/assets/Info.plist" "${BUNDLE_DIR}/Contents/"

# Generate icon if sips is available and we have a PNG
if [ -f "${PROJECT_DIR}/assets/icon.png" ]; then
    echo "Generating .icns from icon.png..."
    ICONSET="${PROJECT_DIR}/target/AppIcon.iconset"
    mkdir -p "${ICONSET}"
    sips -z 16 16     "${PROJECT_DIR}/assets/icon.png" --out "${ICONSET}/icon_16x16.png"      >/dev/null
    sips -z 32 32     "${PROJECT_DIR}/assets/icon.png" --out "${ICONSET}/icon_16x16@2x.png"   >/dev/null
    sips -z 32 32     "${PROJECT_DIR}/assets/icon.png" --out "${ICONSET}/icon_32x32.png"      >/dev/null
    sips -z 64 64     "${PROJECT_DIR}/assets/icon.png" --out "${ICONSET}/icon_32x32@2x.png"   >/dev/null
    sips -z 128 128   "${PROJECT_DIR}/assets/icon.png" --out "${ICONSET}/icon_128x128.png"    >/dev/null
    sips -z 256 256   "${PROJECT_DIR}/assets/icon.png" --out "${ICONSET}/icon_128x128@2x.png" >/dev/null
    sips -z 256 256   "${PROJECT_DIR}/assets/icon.png" --out "${ICONSET}/icon_256x256.png"    >/dev/null
    sips -z 512 512   "${PROJECT_DIR}/assets/icon.png" --out "${ICONSET}/icon_256x256@2x.png" >/dev/null
    sips -z 512 512   "${PROJECT_DIR}/assets/icon.png" --out "${ICONSET}/icon_512x512.png"    >/dev/null
    sips -z 1024 1024 "${PROJECT_DIR}/assets/icon.png" --out "${ICONSET}/icon_512x512@2x.png" >/dev/null
    iconutil -c icns "${ICONSET}" -o "${BUNDLE_DIR}/Contents/Resources/AppIcon.icns"
    rm -rf "${ICONSET}"
fi

# Code sign if identity provided
if [ -n "${IDENTITY}" ]; then
    echo "Signing with: ${IDENTITY}"
    codesign --force --options runtime --sign "${IDENTITY}" "${BUNDLE_DIR}"
    echo "Verifying signature..."
    codesign --verify --verbose=2 "${BUNDLE_DIR}"
fi

echo ""
echo "Done: ${BUNDLE_DIR}"
echo "Size: $(du -sh "${BUNDLE_DIR}" | cut -f1)"
