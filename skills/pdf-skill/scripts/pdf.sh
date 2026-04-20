#!/usr/bin/env bash
# Unified PDF Skill CLI (aligns with SKILL.md Quick Start)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SETUP_SCRIPT="$SCRIPT_DIR/setup.sh"
HTML_SCRIPT="$SCRIPT_DIR/html_to_pdf.js"
LATEX_SCRIPT="$SCRIPT_DIR/compile_latex.py"
PROCESS_SCRIPT="$SCRIPT_DIR/pdf.py"

usage() {
  cat <<'EOF'
Usage: pdf.sh <command> [options]

Commands:
  check [--json]        Run environment diagnostics (setup.sh)
  fix                   Install/repair dependencies (npm playwright, chromium, pip deps)
  html <args...>        Convert HTML to PDF (delegates to html_to_pdf.js)
  latex <args...>       Compile LaTeX with compile_latex.py
  process <args...>     Run python pdf.py process commands (form/extract/pages/...)
EOF
  exit 1
}

append_node_path() {
  local global_root
  if command -v npm &>/dev/null; then
    global_root=$(npm root -g 2>/dev/null || true)
    if [[ -n "$global_root" ]]; then
      if [[ -z "${NODE_PATH:-}" ]]; then
        export NODE_PATH="$global_root"
      elif [[ ":$NODE_PATH:" != *":$global_root:"* ]]; then
        export NODE_PATH="$NODE_PATH:$global_root"
      fi
    fi
  fi
}

cmd_check() {
  set +e
  "$SETUP_SCRIPT" "$@"
  local rc=$?
  set -e
  exit "$rc"
}

cmd_fix() {
  local rc=0

  if command -v npm &>/dev/null; then
    echo "[1/4] Installing local npm dependencies (scripts/)..."
    if ! (cd "$SCRIPT_DIR" && npm install --no-audit --no-fund 2>&1 | tail -3); then
      echo "  Warning: local npm install had issues, trying global..."
    fi

    echo "[2/4] Installing Playwright (global)..."
    if ! npm install -g playwright 2>&1 | tail -3; then
      echo "  Failed to install Playwright via npm."
      rc=3
    fi

    echo "[3/4] Installing Chromium browser..."
    if ! npx playwright install chromium 2>&1 | tail -5; then
      echo "  Failed to install Chromium via Playwright."
      rc=3
    fi
  else
    echo "npm not found; cannot install Playwright automatically."
    rc=2
  fi

  if command -v python3 &>/dev/null; then
    echo "[4/4] Installing Python dependencies (pikepdf, pdfplumber)..."
    if ! python3 -m pip install --user -U pikepdf pdfplumber 2>&1 | tail -3; then
      echo "  Failed to install Python dependencies."
      rc=3
    fi
  else
    echo "python3 not found; cannot install PDF processing dependencies."
    rc=2
  fi

  echo ""
  echo "Checking environment after fix..."
  if "$SETUP_SCRIPT" 2>/dev/null; then
    echo "Environment OK."
  else
    echo "Some issues remain; see output above."
    rc=3
  fi

  exit "$rc"
}

cmd_html() {
  if [[ $# -eq 0 ]]; then
    echo "Missing html command arguments."
    usage
  fi
  if ! command -v node &>/dev/null; then
    echo "node not found; run pdf.sh fix first."
    exit 2
  fi
  append_node_path
  if ! node "$HTML_SCRIPT" "$@"; then
    exit 3
  fi
}

cmd_latex() {
  if [[ $# -eq 0 ]]; then
    echo "Missing latex command arguments."
    usage
  fi
  if ! command -v python3 &>/dev/null; then
    echo "python3 not found; run pdf.sh fix first."
    exit 2
  fi
  if ! python3 "$LATEX_SCRIPT" "$@"; then
    exit 3
  fi
}

cmd_process() {
  if [[ $# -eq 0 ]]; then
    echo "Missing process command arguments."
    usage
  fi
  if ! command -v python3 &>/dev/null; then
    echo "python3 not found; run pdf.sh fix first."
    exit 2
  fi
  if ! python3 "$PROCESS_SCRIPT" "$@"; then
    exit 3
  fi
}

main() {
  if [[ $# -lt 1 ]]; then
    usage
  fi

  local command="$1"
  shift || true

  case "$command" in
    check)
      cmd_check "$@"
      ;;
    fix)
      if [[ $# -ne 0 ]]; then
        usage
      fi
      cmd_fix
      ;;
    html)
      cmd_html "$@"
      ;;
    latex)
      cmd_latex "$@"
      ;;
    process)
      cmd_process "$@"
      ;;
    *)
      usage
      ;;
  esac
}

main "$@"
