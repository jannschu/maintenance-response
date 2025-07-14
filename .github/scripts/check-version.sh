#!/bin/sh
set -e

# Extract version from Cargo.toml
cargo_version=$(grep '^version' Cargo.toml | head -n1 | sed -E 's/version = "(.*)"/\1/')

# Extract version from README.md
readme_version=$(grep -E 'version: v[0-9]+\.[0-9]+\.[0-9]+' README.md | head -n1 | sed -E 's/.*version: v([0-9]+\.[0-9]+\.[0-9]+).*/\1/')

# Get current git tag (assumes running on a tag)
git_tag=$(git describe --tags --exact-match 2>/dev/null || true)

if [ -z "$git_tag" ]; then
  echo "No git tag found for current commit."
  exit 1
fi

if [ "v$cargo_version" != "$git_tag" ]; then
  echo "Version mismatch: Cargo.toml version is $cargo_version, but git tag is $git_tag"
  exit 1
fi

if [ "$cargo_version" != "$readme_version" ]; then
  echo "Version mismatch: Cargo.toml version is $cargo_version, but README.md version is $readme_version"
  exit 1
fi

echo "Version check passed: Cargo.toml ($cargo_version), README.md ($readme_version), and git tag ($git_tag) all match"
