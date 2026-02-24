#!/usr/bin/env bash
# Kiri voice assistant popup launcher
# Bind to a keyboard shortcut in KDE:
#   System Settings → Shortcuts → Custom Shortcuts → Add
#   Command: /home/userx/projects/kiri/scripts/kiri-popup.sh
#
# For Khmer: /home/userx/projects/kiri/scripts/kiri-popup.sh km
#
# KDE Window Rule (for top-left position below panel):
#   System Settings → Window Management → Window Rules → Add
#   Match: Window title = "Kiri"
#   Position → Force: 12, 48
#   Keep Above → Force: Yes
#   Skip Taskbar → Force: Yes
#   No Titlebar → Force: Yes

LANG="${1:-en}"
cd /home/userx/projects/kiri
exec uv run kiri-popup --lang "$LANG" --device GPU
