#!/bin/sh
# Builds review-queue and installs it in an app bundle, so that it can send system
# notifications (macOS allows notifications only from a process with a bundle
# identifier). Then it puts a small script on your PATH that runs the binary in the
# bundle. (A symbolic link is not sufficient: macOS finds no bundle through it.)
#
#   scripts/install-macos.sh
#
# APP_DIR (default ~/Applications) and BIN_DIR (default ~/.cargo/bin) change where the
# bundle and the script go. After the first install, turn on notifications for
# "review-queue" in System Settings → Notifications.
set -eu

cd "$(dirname "$0")/.."
APP_DIR="${APP_DIR:-$HOME/Applications}"
BIN_DIR="${BIN_DIR:-$HOME/.cargo/bin}"
APP="$APP_DIR/review-queue.app"
LSREGISTER=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)

cargo build --release

mkdir -p "$APP/Contents/MacOS" "$BIN_DIR"
cp target/release/review-queue "$APP/Contents/MacOS/review-queue"
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>com.freshtonic.review-queue</string>
  <key>CFBundleName</key><string>review-queue</string>
  <key>CFBundleExecutable</key><string>review-queue</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <!-- No Dock icon: the tool runs in a terminal. -->
  <key>LSUIElement</key><true/>
</dict>
</plist>
PLIST

# An ad-hoc signature: enough for notifications on this computer.
codesign --force --sign - "$APP"
"$LSREGISTER" -f "$APP"
rm -f "$BIN_DIR/review-queue"
cat > "$BIN_DIR/review-queue" <<SCRIPT
#!/bin/sh
exec "$APP/Contents/MacOS/review-queue" "\$@"
SCRIPT
chmod +x "$BIN_DIR/review-queue"

echo "Installed $APP"
echo "Installed $BIN_DIR/review-queue (it runs the binary in the bundle)"
echo "Turn on notifications for review-queue in System Settings → Notifications, if they are off."
