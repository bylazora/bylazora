#!/usr/bin/env bash
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
if [ ! -x "$DIR/batchtxn" ] || [ "$DIR/BATCHTXN.cob" -nt "$DIR/batchtxn" ]; then
  cobc -free -x -O2 -o "$DIR/batchtxn" "$DIR/BATCHTXN.cob"
fi
"$DIR/batchtxn" "$1" "$2" "$3"
