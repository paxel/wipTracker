#!/usr/bin/env bash
# Build a Debian package (.deb) for WipTracker.
#
# The .deb is what makes the taskbar icon work on Linux: it installs the binary, the
# hicolor icon and wiptracker.desktop into the places a desktop actually looks, which a
# bare tarball cannot do. (Wayland has no protocol for a client to set its own icon; the
# compositor matches the window's app_id to <app_id>.desktop instead.)
#
# The runtime Depends are derived from the built binary's own DT_NEEDED libraries, via
# objdump and dpkg -S, so they are right for whatever Debian release you build on — the
# ALSA package, for instance, is libasound2 on bookworm and libasound2t64 on trixie. The
# OpenGL, X11, Wayland and portal libraries are opened at runtime rather than linked, so
# they cannot be detected that way and are listed as Recommends.
#
# Usage:
#   packaging/mkdeb.sh [--version <ver>] [--out <dir>] [--no-build]
set -euo pipefail

OUTDIR="dist"
VERSION=""
DO_BUILD=1

while [ $# -gt 0 ]; do
  case "$1" in
    --version)  VERSION="$2"; shift ;;
    --out)      OUTDIR="$2"; shift ;;
    --no-build) DO_BUILD=0 ;;
    -h|--help)  sed -n '2,17p' "$0"; exit 0 ;;
    *) echo "mkdeb: unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

if [ -z "$VERSION" ]; then
  VERSION=$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')
fi
ARCH=$(dpkg --print-architecture)
BIN="target/release/wiptracker"

if [ "$DO_BUILD" -eq 1 ]; then
  cargo build --release
fi
[ -f "$BIN" ] || { echo "mkdeb: no binary at $BIN" >&2; exit 1; }

STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT

install -Dm755 "$BIN" "$STAGE/usr/bin/wiptracker"
install -Dm644 packaging/wiptracker.desktop "$STAGE/usr/share/applications/wiptracker.desktop"
# Every size the icon generator wrote, read off the files so the list lives only in
# make_icon.py. icon.png is the 512 pixel one; 1024 belongs to the macOS bundle.
install -Dm644 assets/icon.png "$STAGE/usr/share/icons/hicolor/512x512/apps/wiptracker.png"
for source in assets/icon-*.png; do
  [ -f "$source" ] || continue
  size=$(basename "$source" .png)
  size=${size#icon-}
  [ "$size" = 1024 ] && continue
  install -Dm644 "$source" "$STAGE/usr/share/icons/hicolor/${size}x${size}/apps/wiptracker.png"
done
install -Dm644 LICENSE-MIT "$STAGE/usr/share/doc/wiptracker/LICENSE-MIT"
install -Dm644 LICENSE-APACHE "$STAGE/usr/share/doc/wiptracker/LICENSE-APACHE"
install -Dm644 README.md "$STAGE/usr/share/doc/wiptracker/README.md"

# Depends: whatever the binary is actually linked against, mapped to packages.
depends=""
if command -v objdump >/dev/null 2>&1; then
  for lib in $(objdump -p "$BIN" | awk '/NEEDED/ {print $2}'); do
    path=$(ldconfig -p | awk -v l="$lib" '$1 == l {print $NF; exit}' || true)
    [ -n "$path" ] || continue
    # dpkg knows the file by its real path: /lib/... is a symlink into /usr/lib on
    # merged-usr systems and is not itself owned by any package.
    real=$(readlink -f "$path" || true)
    pkg=$(dpkg -S "${real:-$path}" 2>/dev/null | cut -d: -f1 | head -1 || true)
    [ -n "$pkg" ] || pkg=$(dpkg -S "$path" 2>/dev/null | cut -d: -f1 | head -1 || true)
    [ -n "$pkg" ] || continue
    case " $depends " in *" $pkg "*) ;; *) depends="$depends $pkg" ;; esac
  done
fi
# Note: `paste -d', '` would cycle between the comma and the space as two separate
# delimiters, producing "a,b c" — which dpkg rejects.
depends_line=$(echo "$depends" | tr ' ' '\n' | grep -v '^$' | sort -u |
  awk '{ printf "%s%s", separator, $0; separator = ", " } END { print "" }')
[ -n "$depends_line" ] || depends_line="libc6"

mkdir -p "$STAGE/DEBIAN"
cat > "$STAGE/DEBIAN/control" <<CONTROL
Package: wiptracker
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Maintainer: Patrick Zimmer <taum@tuta.io>
Depends: ${depends_line}
Recommends: libgl1, libwayland-client0, libxkbcommon0
Description: One-line always-on-top bar showing the task you are focused on
 WipTracker keeps the task you are working on in a one-line bar above every other
 window, with the rest of what you are juggling on a stack underneath it. Time is
 only collected for the task on top, so the day's numbers describe where your
 attention actually went.
CONTROL

cat > "$STAGE/DEBIAN/postinst" <<'POSTINST'
#!/bin/sh
set -e
command -v gtk-update-icon-cache >/dev/null 2>&1 &&
  gtk-update-icon-cache -f -t /usr/share/icons/hicolor >/dev/null 2>&1 || true
command -v update-desktop-database >/dev/null 2>&1 &&
  update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
exit 0
POSTINST
chmod 755 "$STAGE/DEBIAN/postinst"

mkdir -p "$OUTDIR"
DEB="$OUTDIR/wiptracker_${VERSION}_${ARCH}.deb"
dpkg-deb --build --root-owner-group "$STAGE" "$DEB" >/dev/null
echo "mkdeb: wrote $DEB"
