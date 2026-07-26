#!/usr/bin/env bash
# Downloads the latest ImmichSync .deb release asset and installs it via dpkg
# (so it's tracked by the system package manager, not just a bare binary
# dropped on disk), then runs the interactive setup wizard.
set -euo pipefail

REPO="i-machine-things/ImmichSync"
API_URL="https://api.github.com/repos/${REPO}/releases/latest"

if [ "$(uname -m)" != "x86_64" ]; then
    echo "ImmichSync releases are currently x86_64-only; your architecture is $(uname -m)." >&2
    exit 1
fi

echo "Fetching latest ImmichSync release..."
RELEASE_JSON="$(curl -fsSL "$API_URL")"

DEB_URL="$(printf '%s' "$RELEASE_JSON" \
    | grep -o '"browser_download_url":[[:space:]]*"[^"]*\.deb"' \
    | sed -E 's/.*"(https:[^"]+)"/\1/' \
    | head -n1)"

if [ -z "$DEB_URL" ]; then
    echo "Could not find a .deb asset in the latest release." >&2
    exit 1
fi

TMP_DEB="$(mktemp --suffix=.deb)"
trap 'rm -f "$TMP_DEB"' EXIT

echo "Downloading $DEB_URL"
curl -fsSL "$DEB_URL" -o "$TMP_DEB"

echo "Installing via dpkg (may prompt for your sudo password)..."
sudo dpkg -i "$TMP_DEB" || sudo apt-get install -f -y

echo
echo "ImmichSync installed. Starting the setup wizard..."
immichsync init

read -r -p "Enable the nightly backup schedule now? [Y/n] " reply
reply="${reply:-Y}"
if [[ "$reply" =~ ^[Yy] ]]; then
    immichsync service install
fi
