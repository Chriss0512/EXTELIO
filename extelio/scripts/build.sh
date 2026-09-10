#!/usr/bin/env bash
# Lokaler Build des Add-on-Images. Erfordert Docker mit buildx.
set -euo pipefail
cd "$(dirname "$0")/.."

ARCH="${1:-amd64}"
WITH_FREESWITCH="${WITH_FREESWITCH:-0}"

case "$ARCH" in
  amd64)   BASE="ghcr.io/home-assistant/amd64-base:2026.08.0" ;;
  aarch64) BASE="ghcr.io/home-assistant/aarch64-base:2026.08.0" ;;
  *) echo "Unbekannte Architektur: $ARCH" >&2; exit 1 ;;
esac

VERSION=$(grep -m1 '^version' config.yaml | cut -d'"' -f2)

docker build \
  --build-arg BUILD_FROM="$BASE" \
  --build-arg BUILD_ARCH="$ARCH" \
  --build-arg BUILD_VERSION="$VERSION" \
  --build-arg WITH_FREESWITCH="$WITH_FREESWITCH" \
  --tag "extelio:${VERSION}-${ARCH}" \
  .

echo "Fertig: extelio:${VERSION}-${ARCH} (FreeSWITCH: ${WITH_FREESWITCH})"
