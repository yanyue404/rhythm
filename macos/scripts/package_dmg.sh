#!/bin/zsh
set -euo pipefail

VERSION="${1:-1.0.1}"
SKIP_BUILD="${SKIP_BUILD:-0}"
APP_NAME="Rhythm"
BUNDLE_ID="com.xiao2dou.rhythm"
ICON_BASENAME="Rhythm"
ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist"
APP_DIR="$DIST_DIR/${APP_NAME}.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
ICON_PATH="$ROOT_DIR/assets/${ICON_BASENAME}.icns"
DMG_ROOT="$DIST_DIR/dmg-root"
DMG_PATH="$DIST_DIR/${APP_NAME}-${VERSION}.dmg"

if [[ "$SKIP_BUILD" != "1" ]]; then
  echo "[1/5] Building release binary..."
  swift build -c release --product "$APP_NAME" --package-path "$ROOT_DIR"
else
  echo "[1/5] Skipping build (SKIP_BUILD=1)..."
fi

EXEC_PATH="$(find "$ROOT_DIR/.build" -type f -path "*/release/${APP_NAME}" | head -n 1)"
if [[ -z "$EXEC_PATH" ]]; then
  echo "Release executable not found."
  exit 1
fi

echo "[2/5] Preparing app bundle..."
rm -rf "$APP_DIR" "$DMG_ROOT" "$DMG_PATH"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR" "$DMG_ROOT"
cp "$EXEC_PATH" "$MACOS_DIR/$APP_NAME"
chmod +x "$MACOS_DIR/$APP_NAME"

if [[ -f "$ICON_PATH" ]]; then
  cp "$ICON_PATH" "$RESOURCES_DIR/${ICON_BASENAME}.icns"
else
  echo "Warning: $ICON_PATH not found, app icon will fallback to default."
fi

cat > "$CONTENTS_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleIconFile</key>
    <string>${ICON_BASENAME}</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

echo "[3/5] Applying ad-hoc code signature..."
codesign --force --deep --sign - "$APP_DIR"

echo "[4/5] Creating DMG layout..."
cp -R "$APP_DIR" "$DMG_ROOT/"
ln -s /Applications "$DMG_ROOT/Applications"

echo "[5/5] Building DMG..."
hdiutil create \
  -volname "${APP_NAME} ${VERSION}" \
  -srcfolder "$DMG_ROOT" \
  -ov \
  -format UDZO \
  "$DMG_PATH" >/dev/null

rm -rf "$DMG_ROOT"

echo "Done: $DMG_PATH"
