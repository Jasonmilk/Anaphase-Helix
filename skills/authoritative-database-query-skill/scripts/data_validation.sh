#!/usr/bin/env bash
set -euo pipefail

if [ $# -lt 1 ]; then
  echo "usage: $0 <json_file>"
  exit 1
fi

FILE="$1"
if [ ! -f "$FILE" ]; then
  echo "file not found: $FILE"
  exit 1
fi

jq -e '.' "$FILE" >/dev/null 2>&1 && echo "valid json" || { echo "invalid json"; exit 1; }
