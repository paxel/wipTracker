#!/usr/bin/env sh
# Install the WipTracker icons and .desktop entry into the current user's XDG dirs so
# Wayland (and X11 desktops) show a taskbar icon and a launcher entry. Idempotent.
#
# Wayland has no protocol for a client to set its own window icon: the compositor instead
# matches the window's app_id ("wiptracker") to <app_id>.desktop and uses its Icon= entry.
# That is why the icon compiled into the binary is enough on Windows and macOS but not
# here.
#
# Usage:
#   install-icon.sh                 icons and launcher entry
#   install-icon.sh --autostart     the same, plus start WipTracker at login
#   install-icon.sh --no-autostart  the same, and stop starting it at login
set -eu

autostart=keep
for argument in "$@"; do
  case $argument in
    --autostart) autostart=on ;;
    --no-autostart) autostart=off ;;
    -h | --help)
      sed -n '2,13p' "$0"
      exit 0
      ;;
    *)
      echo "install-icon: unknown argument $argument" >&2
      exit 2
      ;;
  esac
done

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# Works both from a source checkout (packaging/../assets) and from the release tarball,
# where the icons and the .desktop file sit next to this script.
if [ -f "$here/icon.png" ]; then
  icons=$here
else
  icons=$here/../assets
fi
desktop_src="$here/wiptracker.desktop"

data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
config_home="${XDG_CONFIG_HOME:-$HOME/.config}"
desktop_dst="$data_home/applications/wiptracker.desktop"
autostart_dst="$config_home/autostart/wiptracker.desktop"

# Every size the generator wrote, read off the files themselves so the list lives only
# in make_icon.py. The plain icon.png is the 512 pixel one; 1024 is the macOS bundle's
# and no Linux menu asks for it.
mkdir -p "$(dirname "$desktop_dst")"
install_icon() {
  target="$data_home/icons/hicolor/${1}x${1}/apps/wiptracker.png"
  mkdir -p "$(dirname "$target")"
  cp "$2" "$target"
  echo "  $target"
}
for source in "$icons"/icon-*.png; do
  [ -f "$source" ] || continue
  size=$(basename "$source" .png)
  size=${size#icon-}
  [ "$size" = 1024 ] && continue
  install_icon "$size" "$source"
done
[ -f "$icons/icon.png" ] && install_icon 512 "$icons/icon.png"
cp "$desktop_src" "$desktop_dst"

case $autostart in
  on)
    mkdir -p "$(dirname "$autostart_dst")"
    cp "$desktop_src" "$autostart_dst"
    ;;
  off) rm -f "$autostart_dst" ;;
  keep) ;;
esac

# Refresh caches where the tools exist (harmless if they don't).
command -v gtk-update-icon-cache >/dev/null 2>&1 &&
  gtk-update-icon-cache -f -t "$data_home/icons/hicolor" >/dev/null 2>&1 || true
command -v update-desktop-database >/dev/null 2>&1 &&
  update-desktop-database "$data_home/applications" >/dev/null 2>&1 || true
command -v kbuildsycoca6 >/dev/null 2>&1 && kbuildsycoca6 >/dev/null 2>&1 || true

echo "  $desktop_dst"
case $autostart in
  on) echo "  $autostart_dst (starts at login)" ;;
  off) echo "  no longer starting at login" ;;
  keep) echo "Pass --autostart to also start WipTracker at login." ;;
esac
echo "Restart WipTracker; on KDE you may need to log out and in for the taskbar to notice."
