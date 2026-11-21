#!/bin/bash
# Publishes a release: every published crate to crates.io, then the `vVERSION` and
# `clients/go/vVERSION` tags. The `vVERSION` tag starts .github/workflows/release.yml, which builds
# the musl server binary and creates the GitHub release. A crates.io version can never be
# republished, so the script refuses to start unless the tree is exactly origin/main with green
# CI, asks for the version again before it uploads, and skips every step that is already done, so
# a rerun after a failure is safe. --dry-run runs every check and `cargo publish --dry-run`, and
# prints the steps that would change anything.
# Usage: scripts/release.sh VERSION [--dry-run]    (needs `cargo login` and `gh auth login`)
# foundation's ci/release.sh has the same shape; keep the two alike.
set -euo pipefail
usage() { echo "usage: release.sh VERSION [--dry-run], e.g. release.sh 0.1.0" >&2; exit 2; }
version="${1:-}"
dry_run="${2:-}"
# `make release` without VERSION passes --dry-run first.
case "$version" in [0-9]*.[0-9]*.[0-9]*) ;; *) usage ;; esac
[ -z "$dry_run" ] || [ "$dry_run" = --dry-run ] || { echo "unknown option: $dry_run" >&2; exit 2; }
cd "$(git rev-parse --show-toplevel)"

# This repository's part.
workflow=CI
tags=("v$version" "clients/go/v$version")
manifests=(segment-log-*/Cargo.toml)

fail() { echo "release: $*" >&2; exit 1; }
run() { if [ -n "$dry_run" ]; then echo "would run: $*"; else echo "+ $*"; "$@"; fi; }
published() {
	curl -sf -o /dev/null -A "release-script" "https://crates.io/api/v1/crates/$1/$version"
}

git fetch -q --tags origin
[ "$(git branch --show-current)" = main ] || fail "not on main"
[ -z "$(git status --porcelain --untracked-files=no)" ] || fail "uncommitted changes"
head="$(git rev-parse HEAD)"
[ "$head" = "$(git rev-parse origin/main)" ] || fail "HEAD is not origin/main; pull or push first"
cargo_version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
[ "$cargo_version" = "$version" ] || fail "Cargo.toml says $cargo_version, not $version"
grep -q "^## $version " CHANGELOG.md || fail "CHANGELOG.md has no '## $version ' section"
ci="$(gh run list --workflow "$workflow" --commit "$head" --event push --json conclusion -q '.[0].conclusion')"
[ "$ci" = success ] || fail "CI on ${head:0:7} is '${ci:-not run}', not success"
for tag in "${tags[@]}"; do
	if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
		[ "$(git rev-list -n 1 "$tag")" = "$head" ] || fail "tag $tag exists on another commit"
	fi
done

# Every crate without `publish = false`. `cargo publish --workspace` orders them by dependency,
# and its dry run verifies each against the ones before it; a crate already up is excluded.
crates=0
excluded=()
for manifest in "${manifests[@]}"; do
	grep -q '^publish = false' "$manifest" && continue
	crate="$(sed -n 's/^name = "\(.*\)"/\1/p' "$manifest")"
	crates=$((crates + 1))
	if published "$crate"; then
		echo "$crate $version is already on crates.io"
		excluded+=(--exclude "$crate")
	fi
done
if [ "${#excluded[@]}" -eq $((crates * 2)) ]; then
	echo "every crate is already on crates.io"
elif [ -n "$dry_run" ]; then
	cargo publish --workspace --locked --dry-run ${excluded[@]+"${excluded[@]}"}
else
	read -r -p "Publish $version to crates.io? It cannot be undone. Type $version to go on: " answer
	[ "$answer" = "$version" ] || fail "not confirmed; nothing published"
	run cargo publish --workspace --locked ${excluded[@]+"${excluded[@]}"}
fi

for tag in "${tags[@]}"; do
	git rev-parse -q --verify "refs/tags/$tag" >/dev/null || run git tag -a "$tag" -m "$tag" "$head"
done
run git push origin "${tags[@]}"
[ -n "$dry_run" ] || echo "release.yml is building the GitHub release: gh run list --workflow release -L 1"
