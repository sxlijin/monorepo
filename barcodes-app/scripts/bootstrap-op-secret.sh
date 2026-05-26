#!/usr/bin/env bash
#
# Bootstraps the 1Password item that holds the Barcodes App Store Connect credentials.
#
# Creates (or updates) op://roundcolors/barcodes-app with:
#   app_store_connect_key_id          (from $ASC_KEY_ID, else prompt)
#   app_store_connect_issuer_id       (from $ASC_ISSUER_ID, else prompt)
#   app_store_connect_auth_key        (from fastlane/AuthKey.p8)
#   app_store_connect_reviewer_first_name   (from fastlane/metadata/review_information/first_name.txt)
#   app_store_connect_reviewer_last_name    (from .../last_name.txt)
#   app_store_connect_reviewer_phone_number (from .../phone_number.txt)
#
# Values are piped straight from files/env into `op`; nothing is printed to the terminal.
#
# Requires `op` on PATH and signed in (desktop-app integration or `op signin`).
#
# Usage:
#   ./scripts/bootstrap-op-secret.sh            # create/update the item
#   ./scripts/bootstrap-op-secret.sh --prune    # ...then delete the local reviewer .txt files
#
set -euo pipefail

VAULT="roundcolors"
ITEM="barcodes-app"

# Resolve paths relative to the app dir (this script lives in barcodes-app/scripts/).
APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
P8="$APP_DIR/fastlane/AuthKey.p8"
REVIEW_DIR="$APP_DIR/fastlane/metadata/review_information"

PRUNE=false
[[ "${1:-}" == "--prune" ]] && PRUNE=true

die() { echo "error: $*" >&2; exit 1; }

command -v op >/dev/null || die "op CLI not found on PATH (the repo pins it via mise: 'mise install')."
op whoami >/dev/null 2>&1 || die "op is not signed in. Enable 1Password desktop CLI integration or run 'op signin'."

[[ -f "$P8" ]] || die "missing $P8"
for f in first_name last_name phone_number; do
  [[ -f "$REVIEW_DIR/$f.txt" ]] || die "missing $REVIEW_DIR/$f.txt"
done

# Identifiers (not secret, but kept out of the repo): env var, else prompt.
KEY_ID="${ASC_KEY_ID:-}"
ISSUER_ID="${ASC_ISSUER_ID:-}"
[[ -n "$KEY_ID" ]]    || read -rp "App Store Connect Key ID: " KEY_ID
[[ -n "$ISSUER_ID" ]] || read -rp "App Store Connect Issuer ID: " ISSUER_ID

# Field assignments shared by create + edit. .p8 is concealed (password type).
FIELDS=(
  "app_store_connect_key_id[text]=$KEY_ID"
  "app_store_connect_issuer_id[text]=$ISSUER_ID"
  "app_store_connect_auth_key[password]=$(cat "$P8")"
  "app_store_connect_reviewer_first_name[text]=$(cat "$REVIEW_DIR/first_name.txt")"
  "app_store_connect_reviewer_last_name[text]=$(cat "$REVIEW_DIR/last_name.txt")"
  "app_store_connect_reviewer_phone_number[text]=$(cat "$REVIEW_DIR/phone_number.txt")"
)

if op item get "$ITEM" --vault "$VAULT" >/dev/null 2>&1; then
  echo "Updating existing item $VAULT/$ITEM ..."
  op item edit "$ITEM" --vault "$VAULT" "${FIELDS[@]}" >/dev/null
else
  echo "Creating item $VAULT/$ITEM ..."
  op item create --category "API Credential" --vault "$VAULT" --title "$ITEM" "${FIELDS[@]}" >/dev/null
fi

# Verify by listing field labels only (never values).
echo "Fields now on the item:"
op item get "$ITEM" --vault "$VAULT" --format json \
  | grep -o '"label": *"app_store_connect_[^"]*"' | sed 's/.*"\(app_store_connect_[^"]*\)"/  - \1/' | sort -u

if $PRUNE; then
  rm -f "$REVIEW_DIR/first_name.txt" "$REVIEW_DIR/last_name.txt" "$REVIEW_DIR/phone_number.txt"
  echo "Pruned local reviewer files (1Password is now the source of truth)."
else
  echo "Local reviewer files left in place. Re-run with --prune to delete them once you've verified."
fi
