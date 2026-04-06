#!/usr/bin/env bash
set -euo pipefail

LOCAL_ROOT="${LOCAL_ROOT:-./}"
REMOTE="${REMOTE:-agmirabella@login.spc.cica.es}"
REMOTE_ROOT="${REMOTE_ROOT:-/home/agmirabella/monte-carlo/beamdpr}"
EXCLUDES_FILE="${EXCLUDES_FILE:-.rsyncignore}"
SSH_PORT="${SSH_PORT:-22}"
RSYNC_BIN="${RSYNC_BIN:-/usr/local/bin/rsync}"

print_usage() {
  cat <<'USAGE'
Usage: ./push.sh [options]

Options:
  --dry-run, -n     Preview the transfer without copying files.
  --no-dry-run      Force a live transfer even if --dry-run was defaulted elsewhere.
  -h, --help        Show this help and exit.
  -- ...            Any additional flags after "--" are passed through to rsync.

You can also override configuration via environment variables, e.g.: 
  RSYNC_BIN=/opt/homebrew/bin/rsync ./push.sh --dry-run
USAGE
}

DRY_RUN=0
RSYNC_EXTRA=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run|-n)
      DRY_RUN=1
      ;;
    --no-dry-run)
      DRY_RUN=0
      ;;
    -h|--help)
      print_usage
      exit 0
      ;;
    --)
      shift
      RSYNC_EXTRA+=("$@")
      break
      ;;
    *)
      RSYNC_EXTRA+=("$1")
      ;;
  esac
  shift || true
done

if ! command -v "$RSYNC_BIN" >/dev/null 2>&1; then
  if RSYNC_FALLBACK=$(command -v rsync 2>/dev/null); then
    RSYNC_BIN="$RSYNC_FALLBACK"
  else
    echo "Error: rsync binary not found" >&2
    exit 1
  fi
fi

if [[ ! -d "$LOCAL_ROOT" ]]; then
  echo "Error: LOCAL_ROOT '$LOCAL_ROOT' does not exist or is not a directory." >&2
  exit 1
fi

SOURCE="${LOCAL_ROOT%/}/"
TARGET="${REMOTE}:${REMOTE_ROOT%/}"

RSYNC_OPTS=(
  -az
  --info=stats2,progress2
  --human-readable
  --rsh="ssh -p ${SSH_PORT}"
  --filter=':- .gitignore'
  --exclude='.git/'
  --prune-empty-dirs
  --partial
)

if [[ -f "$EXCLUDES_FILE" ]]; then
  RSYNC_OPTS+=("--exclude-from=${EXCLUDES_FILE}")
fi

if (( DRY_RUN )); then
  RSYNC_OPTS+=(--dry-run)
  echo "Running rsync in dry-run mode..."
else
  echo "Running rsync (live transfer)..."
fi

set -x
"$RSYNC_BIN" "${RSYNC_OPTS[@]}" "${RSYNC_EXTRA[@]}" "$SOURCE" "$TARGET"
set +x
