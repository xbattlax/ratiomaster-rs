#!/bin/bash
set -euo pipefail

APP_NAME="RatioMaster"
VERSION="0.1.0"
DMG_NAME="${APP_NAME}-${VERSION}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
APP_BUNDLE="${PROJECT_DIR}/target/${APP_NAME}.app"
DMG_DIR="${PROJECT_DIR}/target/dmg"
DMG_PATH="${PROJECT_DIR}/target/${DMG_NAME}.dmg"

if [ ! -d "${APP_BUNDLE}" ]; then
    echo "Error: ${APP_BUNDLE} not found. Run scripts/bundle-macos.sh first."
    exit 1
fi

echo "Creating DMG installer..."

# Clean
rm -rf "${DMG_DIR}" "${DMG_PATH}"
mkdir -p "${DMG_DIR}"

# Copy app
cp -R "${APP_BUNDLE}" "${DMG_DIR}/"

# Create Applications symlink
ln -s /Applications "${DMG_DIR}/Applications"

# Create DMG
hdiutil create -volname "${APP_NAME}" \
    -srcfolder "${DMG_DIR}" \
    -ov -format UDZO \
    "${DMG_PATH}"

# Clean staging
rm -rf "${DMG_DIR}"

echo ""
echo "Done: ${DMG_PATH}"
echo "Size: $(du -sh "${DMG_PATH}" | cut -f1)"
