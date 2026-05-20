#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_DIR/target/appimage"
APP_DIR="$BUILD_DIR/MiaoMC.AppDir"

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

if [ -f "$SCRIPT_DIR/miao-mc.png" ]; then
    cp "$SCRIPT_DIR/miao-mc.png" "$APP_DIR/usr/share/icons/hicolor/256x256/apps/"
    cp "$SCRIPT_DIR/miao-mc.png" "$APP_DIR/"
else
    echo "Warning: No icon found at appimage/miao-mc.png, generating placeholder..."
    convert -size 256x256 xc:'#4a9eff' -gravity center \
        -pointsize 80 -fill white -annotate 0 'M' \
        "$APP_DIR/miao-mc.png" 2>/dev/null || \
    printf '\x89PNG\r\n\x1a\n' > "$APP_DIR/miao-mc.png"
    cp "$APP_DIR/miao-mc.png" "$APP_DIR/usr/share/icons/hicolor/256x256/apps/"
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
ARCH=$(uname -m) "$APPIMAGETOOL" "$APP_DIR" "$BUILD_DIR/MiaoMC-$ARCH.AppImage"

echo "Done: $BUILD_DIR/MiaoMC-$ARCH.AppImage"
