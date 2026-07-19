#!/usr/bin/env bash
# Package OmniNote for macOS: cargo-bundle → OmniNote.app → dist/OmniNote.dmg.
#
# Usage: scripts/make-dmg.sh
# Requires: cargo-bundle (cargo install cargo-bundle), hdiutil (macOS built-in).
#
# The .dmg is the drag-to-Applications installer: it carries OmniNote.app and
# an /Applications symlink. Output lands in dist/ (gitignored — artifacts are
# never committed).
set -euo pipefail

cd "$(dirname "$0")/.."

# Honour a shared out-of-tree target-dir if the local .cargo/config.toml sets
# one (this repo does — the root doubles as an OmniNote vault, so an in-tree
# target/ would be walked by the vault scanner).
TARGET_DIR="$(cargo metadata --format-version 1 --no-deps 2>/dev/null \
  | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"

echo "==> cargo bundle --release (target-dir: $TARGET_DIR)"
cargo bundle --release --package omninote-gui

APP="$TARGET_DIR/release/bundle/osx/OmniNote.app"
[ -d "$APP" ] || { echo "error: $APP not found after cargo bundle" >&2; exit 1; }

VERSION="$(cargo metadata --format-version 1 --no-deps 2>/dev/null \
  | python3 -c 'import json,sys; m=json.load(sys.stdin); print(next(p["version"] for p in m["packages"] if p["name"]=="omninote-gui"))')"
DMG="dist/OmniNote-$VERSION.dmg"

echo "==> staging $DMG"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"

mkdir -p dist
rm -f "$DMG"
hdiutil create -volname "OmniNote" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null

echo "==> done: $DMG"
du -sh "$DMG"
