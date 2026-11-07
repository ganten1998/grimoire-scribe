#!/bin/sh
# What must be true of the tree before it is tagged as a release (docs/RELEASING.md, Before the tag).
# Run by hand on the release commit, and by the release workflow on the tag it was started for.
# It reads files and git's history only: no build, no network, no git write.
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

# docs/whatsnew/VERSION.md: every release has its What's new, at least a basic one (the owner's rule
# of 2026-11-05: "every update with changes by default has at least a basic what's new"), the notes
# the first start after the update opens by itself (since 2.1.0 every version's notes are a file
# there; the app builds them in). This version's file is there, has notes (a line besides its "# "
# title and the {signature} line), and they are not the release before's again, spaces and line
# breaks aside. The release before is the newest release tag (vMAJOR.MINOR.PATCH[-beta.N|-rc.N])
# that is an ancestor of HEAD and not on HEAD itself (that one is this release's own tag); its notes
# are its docs/whatsnew/ file for its version, or docs/WHATSNEW.md for a release before 2.1.0 (the
# single file of then). Where the history or the tags cannot tell (a shallow clone, a clone without
# tags, the first release), the comparison says so and does not fail for that; the release workflow
# fetches both before it runs this.
note=docs/whatsnew/$VERSION.md
norm() {  # norm: the text on stdin with each run of spaces and line breaks one space, none at the ends
    tr -s '[:space:]' ' ' | sed -e 's/^ //' -e 's/ $//'
}
release_re='[0-9]+\.[0-9]+\.[0-9]+(-(beta|rc)\.(0|[1-9][0-9]{0,2}))?'
if [ ! -f "$note" ]; then
    bad "$note is missing: write this version's What's new, at least its title and the user-visible changes in short bullets (docs/RELEASING.md, The bump commit)"
elif ! grep -v -e '^[[:space:]]*$' -e '^[[:space:]]*# ' -e '^[[:space:]]*{signature}[[:space:]]*$' "$note" | grep -q .; then
    bad "$note has no notes: write this version's user-visible changes in short bullets under its title (docs/RELEASING.md, The bump commit)"
elif ! git rev-parse --verify -q HEAD >/dev/null 2>&1; then
    echo "release-preflight: $note not compared: no git history here to compare it with"
else
    own=$(git tag --points-at HEAD 2>/dev/null)
    prev='' best=''
    for t in $(git tag --merged HEAD --list 'v*' 2>/dev/null); do
        printf '%s\n' "$t" | grep -qE "^v$release_re\$" || continue
        if printf '%s\n' "$own" | grep -qxF "$t"; then continue; fi
        n=$(git rev-list --count "$t..HEAD" 2>/dev/null) || continue
        if [ -z "$best" ] || [ "$n" -lt "$best" ]; then best=$n prev=$t; fi
    done
    if [ -z "$prev" ]; then
        if [ "$(git rev-parse --is-shallow-repository 2>/dev/null)" = true ]; then
            why="a shallow clone; git fetch --unshallow --tags, then run this again"
        else
            why="no earlier release tag among its ancestors; git fetch --tags, then run this again"
        fi
        echo "release-preflight: $note not compared: could not tell the previous release ($why)"
    else
        was=''
        for f in "docs/whatsnew/${prev#v}.md" docs/WHATSNEW.md; do
            if git cat-file -e "$prev:$f" 2>/dev/null; then was=$f; break; fi
        done
        if [ -z "$was" ]; then
            echo "release-preflight: $note not compared: $prev has no What's new"
        elif [ "$(git show "$prev:$was" | norm)" = "$(norm < "$note")" ]; then
            bad "$note is $prev's What's new again ($was there): write this version's, at least its title and the user-visible changes in short bullets (docs/RELEASING.md, The bump commit)"
        else
            echo "release-preflight: $note is new since $prev ($was there)"
        fi
    fi
fi

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
