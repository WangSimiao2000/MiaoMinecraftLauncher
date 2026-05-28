#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_DIR/target/appimage"
APP_DIR="$BUILD_DIR/MMCL.AppDir"

echo "Building release binary..."
cargo build --release -p miao-gui --manifest-path "$PROJECT_DIR/Cargo.toml"

echo "Preparing AppDir..."
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/usr/bin"
mkdir -p "$APP_DIR/usr/share/icons/hicolor/256x256/apps"
mkdir -p "$APP_DIR/usr/share/applications"

cp "$PROJECT_DIR/target/release/miao-gui" "$APP_DIR/usr/bin/"
cp "$SCRIPT_DIR/miao-mc.desktop" "$APP_DIR/usr/share/applications/"
cp "$SCRIPT_DIR/miao-mc.desktop" "$APP_DIR/"

ICON_SRC="$PROJECT_DIR/assets/icon.png"
if [ -f "$ICON_SRC" ]; then
    cp "$ICON_SRC" "$APP_DIR/usr/share/icons/hicolor/256x256/apps/miao-mc.png"
    cp "$ICON_SRC" "$APP_DIR/miao-mc.png"
else
    echo "Error: Icon not found at assets/icon.png"
    exit 1
fi

cat > "$APP_DIR/AppRun" << 'EOF'
#!/bin/bash
SELF_DIR="$(dirname "$(readlink -f "$0")")"
exec "$SELF_DIR/usr/bin/miao-gui" "$@"
EOF
chmod +x "$APP_DIR/AppRun"

APPIMAGETOOL="$BUILD_DIR/appimagetool"
if [ ! -f "$APPIMAGETOOL" ]; then
    echo "Downloading appimagetool..."
    ARCH=$(uname -m)
    curl -Lo "$APPIMAGETOOL" \
        "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$ARCH.AppImage"
    chmod +x "$APPIMAGETOOL"
fi

echo "Building AppImage..."
ARCH=$(uname -m) "$APPIMAGETOOL" "$APP_DIR" "$BUILD_DIR/mmcl-linux-$ARCH.AppImage"

echo "Done: $BUILD_DIR/mmcl-linux-$ARCH.AppImage"
