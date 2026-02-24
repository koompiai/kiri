#!/usr/bin/env bash
# Kiri voice assistant popup launcher
# Usage: kiri-popup.sh [language] [mode]
#   language: en (default) or km
#   mode: type (default) or notes

LANG="${1:-en}"
MODE="${2:-type}"
cd /home/userx/projects/kiri
export LD_PRELOAD=/usr/lib/libgtk4-layer-shell.so
exec uv run kiri-popup --lang "$LANG" --mode "$MODE" --device GPU
