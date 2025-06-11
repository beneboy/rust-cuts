#!/bin/bash
set -e

echo "Building RustCuts GUI..."
cargo build --release --bin rcg --target-dir ./workspace-target

echo "Preparing bundle directory..."
mkdir -p crates/gui/target/release

echo "Copying binary..."
cp workspace-target/release/rcg crates/gui/target/release/rcg

echo "Creating macOS app bundle manually..."
APP_DIR="crates/gui/target/RustCuts.app"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

echo "Copying binary to app bundle..."
cp workspace-target/release/rcg "$APP_DIR/Contents/MacOS/rcg"

echo "Creating Info.plist..."
cat > "$APP_DIR/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDisplayName</key>
    <string>RustCuts</string>
    <key>CFBundleExecutable</key>
    <string>rcg</string>
    <key>CFBundleIdentifier</key>
    <string>com.rustcuts.app</string>
    <key>CFBundleName</key>
    <string>RustCuts</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.2.0</string>
    <key>CFBundleVersion</key>
    <string>0.2.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2024</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
</dict>
</plist>
EOF

echo "✅ RustCuts.app created successfully!"
echo "📦 App bundle location: ${APP_DIR}"
echo "🚀 You can now drag this to Applications or double-click to run!"