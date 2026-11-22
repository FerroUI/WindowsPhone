#!/usr/bin/env bash
set -euo pipefail
PHONE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PHONE_ROOT"
export FERROUI_BROWSER_WORKSPACE="$PHONE_ROOT"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"
export CARGO_PROFILE_BROWSER_CODEGEN_UNITS="${CARGO_PROFILE_BROWSER_CODEGEN_UNITS:-16}"
FerroUI/scripts/build-browser.sh phone-browser "$@"
PHONE_SITE="${CARGO_TARGET_DIR:-$PHONE_ROOT/target}/browser/phone-browser"
node scripts/prepare-site.mjs "$PHONE_SITE"
