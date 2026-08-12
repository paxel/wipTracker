#!/usr/bin/env sh
# Install the WipTracker icon and .desktop entry into the current user's XDG dirs so
# Wayland (and X11 desktops) show a taskbar icon. Idempotent.
#
# Wayland has no protocol for a client to set its own window icon: the compositor instead
# matches the window's app_id ("wiptracker") to <app_id>.desktop and uses its Icon= entry.
# That is why the icon compiled into the binary is enough on Windows and macOS but not
# here.
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# Works both from a source checkout (packaging/../assets) and from the release tarball,
# where the icon and the .desktop file sit next to this script.
if [ -f "$here/icon.png" ]; then
  icon_src="$here/icon.png"
  desktop_src="$here/wiptracker.desktop"
else
  icon_src="$here/../assets/icon.png"
  desktop_src="$here/wiptracker.desktop"
fi

data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
icon_dst="$data_home/icons/hicolor/512x512/apps/wiptracker.png"
desktop_dst="$data_home/applications/wiptracker.desktop"

mkdir -p "$(dirname "$icon_dst")" "$(dirname "$desktop_dst")"
cp "$icon_src" "$icon_dst"
cp "$desktop_src" "$desktop_dst"

# Refresh caches where the tools exist (harmless if they don't).
command -v gtk-update-icon-cache >/dev/null 2>&1 &&
  gtk-update-icon-cache -f -t "$data_home/icons/hicolor" >/dev/null 2>&1 || true
command -v update-desktop-database >/dev/null 2>&1 &&
  update-desktop-database "$data_home/applications" >/dev/null 2>&1 || true
command -v kbuildsycoca6 >/dev/null 2>&1 && kbuildsycoca6 >/dev/null 2>&1 || true

echo "Installed:"
echo "  $icon_dst"
echo "  $desktop_dst"
echo "Restart WipTracker; on KDE you may need to log out and in for the taskbar to notice."
