#!/usr/bin/env bash
# Install AnythingView to ~/.local (user-local, no sudo required).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
APP_DIR="$PREFIX/share/applications"
ICON_DIR="$PREFIX/share/icons/hicolor/scalable/apps"
MIME_DIR="$PREFIX/share/mime/packages"

echo "→ Building release binary…"
cargo build --release

echo "→ Installing to $PREFIX"
install -d "$BIN_DIR" "$APP_DIR" "$ICON_DIR" "$MIME_DIR"

install -m 0755 target/release/anythingview "$BIN_DIR/anythingview"
install -m 0644 data/anythingview.desktop "$APP_DIR/anythingview.desktop"

if [ -f data/anythingview.svg ]; then
    install -m 0644 data/anythingview.svg "$ICON_DIR/anythingview.svg"
fi

if [ -f data/anythingview.xml ]; then
    install -m 0644 data/anythingview.xml "$MIME_DIR/anythingview.xml"
    if command -v update-mime-database &>/dev/null; then
        update-mime-database "$PREFIX/share/mime" || true
    fi
fi

if command -v update-desktop-database &>/dev/null; then
    update-desktop-database "$APP_DIR" || true
fi

if command -v gtk-update-icon-cache &>/dev/null; then
    gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" 2>/dev/null || true
fi

echo "✓ Installed: $BIN_DIR/anythingview"
echo "  Desktop:   $APP_DIR/anythingview.desktop"
echo ""
echo "Make sure $BIN_DIR is in your PATH, then run: anythingview <file>"
