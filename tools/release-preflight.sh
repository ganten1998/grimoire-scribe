#!/bin/sh
# What must be true of the tree before it is tagged as a release (docs/RELEASING.md, Before the tag).
# Run by hand on the release commit, and by the release workflow on the tag it was started for.
# It reads files only: no build, no network, no git write.
#
# usage: tools/release-preflight.sh [TAG]
#   TAG  the release's tag; when given it must be v<VERSION> (the workflow passes the release's).
# Exit 0 when every check holds, 1 when any fails (all are listed).
set -u
cd "$(dirname "$0")/.." || exit 1
fail=0
bad() {  # bad MESSAGE: one problem
    echo "release-preflight: $1"
    fail=1
}

VERSION=$(cat VERSION 2>/dev/null) || VERSION=
if ! printf '%s\n' "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-(beta|rc)\.(0|[1-9][0-9]{0,2}))?$'; then
    bad "VERSION ($VERSION) is not MAJOR.MINOR.PATCH[-beta.N|-rc.N] with N from 0 to 999"
    echo "RELEASE PREFLIGHT FAILED"
    exit 1
fi
echo "release-preflight: version $VERSION"

if [ $# -ge 1 ] && [ "$1" != "v$VERSION" ]; then
    bad "the tag $1 is not v$VERSION"
fi

# CHANGELOG.md: this version has a section with its date, not "(unreleased)", and nothing in the
# file is still marked as unwritten.
escaped=$(printf '%s' "$VERSION" | sed 's/[.]/\\./g')
if ! grep -qE "^## $escaped \([0-9]{4}-[0-9]{2}-[0-9]{2}\)$" CHANGELOG.md; then
    bad "CHANGELOG.md has no heading '## $VERSION (YYYY-MM-DD)' (an unreleased one reads '(unreleased)')"
fi
if grep -n 'RELEASE-PLACEHOLDER' CHANGELOG.md; then
    bad "CHANGELOG.md still has the placeholders above: write each entry, then delete its line"
fi

# README.md: a pre-release names itself in its banner; a stable release has no pre-release banner.
case $VERSION in
*-*)
    grep -qF "Pre-release $VERSION." README.md || bad "README.md's banner does not read 'Pre-release $VERSION.'" ;;
*)
    if grep -q 'Pre-release [0-9]' README.md; then bad "README.md still has a pre-release banner"; fi ;;
esac

# docs/TESTING.md: a section for this version (what was checked on a Kindle, and what was not).
grep -qE "^## $escaped( |$)" docs/TESTING.md || bad "docs/TESTING.md has no '## $VERSION' section"

# The tree is what is tagged: no uncommitted change to a tracked file.
if [ -n "$(git status --porcelain --untracked-files=no 2>/dev/null)" ]; then
    bad "tracked files have uncommitted changes (git status)"
fi

if [ $fail = 0 ]; then
    echo "RELEASE PREFLIGHT PASSED"
else
    echo "RELEASE PREFLIGHT FAILED"
    exit 1
fi
