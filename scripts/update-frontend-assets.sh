#!/usr/bin/env bash
# Regenerates the vendored front-end assets embedded into the binary via `include_str!`
# in src/assets.rs (static/css/app.css, static/js/htmx.min.js). Run manually whenever you
# want to bump htmx or Tailwind, review the diff (git diff static/), then commit.
#
# Usage: scripts/update-frontend-assets.sh [htmx_version] [tailwind_version]
set -euo pipefail

HTMX_VERSION="${1:-2.0.4}"
TAILWIND_VERSION="${2:-v4.3.3}"

cd "$(dirname "$0")/.."

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) tw_asset="tailwindcss-linux-x64" ;;
    Linux-aarch64 | Linux-arm64) tw_asset="tailwindcss-linux-arm64" ;;
    Darwin-x86_64) tw_asset="tailwindcss-macos-x64" ;;
    Darwin-arm64) tw_asset="tailwindcss-macos-arm64" ;;
    *)
        echo "Unsupported platform: $(uname -s)-$(uname -m)" >&2
        exit 1
        ;;
esac

echo "==> Downloading htmx ${HTMX_VERSION}"
curl -sL --fail -o static/js/htmx.min.js \
    "https://unpkg.com/htmx.org@${HTMX_VERSION}/dist/htmx.min.js"

mkdir -p .tools
if [[ -f .tools/tailwindcss ]]; then
    echo "==> Using existing Tailwind CLI (${tw_asset})"
else
    echo "==> Fetching Tailwind CLI ${TAILWIND_VERSION} (${tw_asset})"
    curl -sL --fail -o .tools/tailwindcss \
        "https://github.com/tailwindlabs/tailwindcss/releases/download/${TAILWIND_VERSION}/${tw_asset}"
fi
chmod +x .tools/tailwindcss

echo "==> Building static/css/app.css"
./.tools/tailwindcss -i ./assets/tailwind.css -o ./static/css/app.css --minify

echo "==> Done. Review the diff (git diff static/) and commit if it looks right."
