#!/bin/sh
set -e

# Extract version from Cargo.toml
cargo_version=$(grep '^version' Cargo.toml | head -n1 | sed -E 's/version = "(.*)"/\1/')

# Get current git tag (assumes running on a tag)
git_tag=$(git describe --tags --exact-match 2>/dev/null || true)

if [ -z "$git_tag" ]; then
  echo "No git tag found for current commit."
  exit 1
fi

if [ "$cargo_version" != "$git_tag" ]; then
  echo "Version mismatch: Cargo.toml version is $cargo_version, but git tag is $git_tag"
  exit 1
fi

echo "Cargo.toml version matches git tag: $cargo_version"
