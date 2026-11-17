#!/bin/sh
# Runs a command inside an X11 session: Xvfb with openbox as the EWMH window manager.
# The X11 desktop tests (tests/x11_desktop.rs) need one and fail without it. CI's Rust gate
# runs the tests and the coverage measurement through this script (ci.yml, test-wrapper);
# the release workflow does the same for the tests on the tagged commit.
#
# The screen is tall enough for the menu to open below the centred bar, where the tests
# click. xte, the pointer the tests use, comes from the xautomation package.
#
# Usage: scripts/with-desktop.sh cargo test --locked --all-features
set -eu
exec xvfb-run -a -s '-screen 0 1920x1600x24' sh -c 'openbox & sleep 2; exec "$@"' with-desktop "$@"
