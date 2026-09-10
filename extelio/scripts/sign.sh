#!/usr/bin/env bash
# Signiert Release-Artefakte. Die Signaturidentitaet stellt der Betreiber.
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v cosign >/dev/null 2>&1; then
  echo "cosign ist nicht installiert." >&2
  exit 1
fi

IMAGE="${1:?Nutzung: sign.sh <image-referenz>}"

# Keyless-Signatur ueber OIDC. Ohne gueltige Identitaet bricht der Vorgang ab,
# statt ein unsigniertes Artefakt als signiert auszugeben.
COSIGN_EXPERIMENTAL=1 cosign sign --yes "$IMAGE"

for sbom in dist/sbom-*.cdx.json; do
  [ -e "$sbom" ] || continue
  COSIGN_EXPERIMENTAL=1 cosign attest --yes --type cyclonedx --predicate "$sbom" "$IMAGE"
done

echo "Signatur und Attestierung abgeschlossen."
