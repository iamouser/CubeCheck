#!/bin/bash
# Double-click helper for macOS Finder.
cd "$(dirname "$0")" || exit 1
if [ -x "./cubecheck" ]; then
  exec ./cubecheck "$@"
fi
if [ -f "./cubecheck.sh" ]; then
  chmod +x ./cubecheck.sh 2>/dev/null || true
  exec ./cubecheck.sh "$@"
fi
echo "CubeCheck: положите cubecheck или cubecheck.sh рядом с этим файлом." >&2
exit 1
