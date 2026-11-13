#!/usr/bin/env bash
# One-time setup for this fork's signed launcher updates: makes an Ed25519 key, stores its private half as
# the RELEASE_SIGNING_KEY secret of the fork's repository (never printed, never written to disk), and appends
# the public half to crates/horizon-release-sign/release-keys.txt, which launchers build in. Run it once,
# commit release-keys.txt, then release (.github/workflows/release-elyby.yml). Needs openssl and a logged-in gh.
#   tools/fork_signing_key.sh [owner/repo]
set -euo pipefail
repo=${1:-Sobble2546/horizon-ely.by}
keys="$(dirname "$0")/../crates/horizon-release-sign/release-keys.txt"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
umask 077
openssl genpkey -algorithm ed25519 -outform DER -out "$work/key"
public=$(openssl pkey -inform DER -in "$work/key" -pubout -outform DER | tail -c 32 | base64)
base64 -w0 < "$work/key" 2>/dev/null | gh secret set RELEASE_SIGNING_KEY --repo "$repo"
printf '# Signs %s'"'"'s launcher releases; the private half is that repository'"'"'s RELEASE_SIGNING_KEY secret.\n%s\n' "$repo" "$public" >> "$keys"
echo "Secret set on $repo; public key added to $keys. Commit that file before the next release."
