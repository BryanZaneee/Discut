#!/bin/sh
# Explicit offline fixtures only. Each process launched here is stopped after capture.
set -eu
cd "$(dirname "$0")/.."
binary="${1:-target/debug/serein}"
mkdir -p docs/screenshots
preview_pid=
trap 'if [ -n "$preview_pid" ]; then kill "$preview_pid" 2>/dev/null || :; fi' EXIT HUP INT TERM
capture() {
  destination="$PWD/docs/screenshots/$1.png"
  shift
  rm -f "$destination"
  "$binary" --demo --demo-light "$@" "--demo-screenshot=$destination" &
  preview_pid=$!
  count=0
  while [ ! -s "$destination" ] && kill -0 "$preview_pid" 2>/dev/null; do
    count=$((count + 1))
    if [ "$count" -gt 30 ]; then echo "Capture timed out: $destination" >&2; exit 1; fi
    sleep 1
  done
  test -s "$destination"
  kill "$preview_pid" 2>/dev/null || :
  wait "$preview_pid" 2>/dev/null || :
  preview_pid=
}
capture discut-chat
capture discut-call --demo-call
capture discut-selection --demo-group-dm --demo-settings=chat
