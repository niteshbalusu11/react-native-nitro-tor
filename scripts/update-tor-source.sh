#!/usr/bin/env bash

set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: $0 <tor-version>" >&2
  exit 2
fi

tor_version="$1"
tor_tag="tor-${tor_version}"
project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
tor_destination="${project_root}/crates/libtor-sys/vendor/tor"
patch_directory="${project_root}/crates/libtor-sys/patches"
work_directory="$(mktemp -d -t react-native-nitro-tor-update.XXXXXX)"
checkout_directory="${work_directory}/tor"

cleanup() {
  rm -rf "$work_directory"
}
trap cleanup EXIT

git clone \
  --branch "$tor_tag" \
  --depth 1 \
  https://gitlab.torproject.org/tpo/core/tor.git \
  "$checkout_directory"

git -C "$checkout_directory" tag --verify "$tor_tag"

find "$checkout_directory" \( -name Cargo.toml -o -name Cargo.lock \) -delete

for patch in "$patch_directory"/tor-*.patch; do
  git -C "$checkout_directory" apply --check -p1 "$patch"
done

rsync \
  --archive \
  --delete \
  --exclude .git \
  --exclude .gitignore \
  "$checkout_directory"/ \
  "$tor_destination"/

tor_commit="$(git -C "$checkout_directory" rev-parse HEAD)"

echo "Imported Tor ${tor_version} (${tor_commit})."
echo "Next: update crates/libtor-sys/vendor/UPSTREAM.md and crate version metadata."
echo "Then run the targeted and full build commands documented in docs/research/tor-vendoring-and-upgrade-plan.md."
