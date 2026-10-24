#!/bin/sh
set -eu
. "$(dirname "$0")/common.sh"
platform
require_closed
# Do not create grimoire/ here: the payload must be able to migrate an existing quire/ on first
# launch. Installer backups/lock are temporary; final launcher staging stays beside Documents.
mkdir -p "$DOCS" "$TMP"
WORK=$TMP/grimoire.kpm-install
if ! mkdir "$WORK"; then echo 'Another Grimoire installation is pending.' >&2; exit 1; fi
# Kindle's fsp refuses cross-directory moves into the real documents/ folder.
# Stage beside each destination so the final rename stays within that directory.
STAGE=$DOCS/.grimoire-kpm-$$
for name in Grimoire.sh 'Grimoire - reset pen calibration.sh'; do
    if [ -e "$STAGE-$name" ]; then
        rmdir "$WORK"
        echo 'A launcher staging file already exists; keeping it.' >&2
        exit 1
    fi
done
ok=no
changed=no
cleanup() {
    if [ "$ok" != yes ] && [ "$changed" = yes ]; then
        for name in Grimoire.sh 'Grimoire - reset pen calibration.sh'; do
            if [ -f "$WORK/old/$name" ]; then cp -f "$WORK/old/$name" "$DOCS/$name"; else rm -f "$DOCS/$name"; fi
        done
    fi
    for name in Grimoire.sh 'Grimoire - reset pen calibration.sh'; do rm -f "$STAGE-$name"; done
    rm -rf "$WORK"
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM
mkdir "$WORK/old"
for name in Grimoire.sh 'Grimoire - reset pen calibration.sh'; do
    if [ -e "$DOCS/$name" ]; then
        # An existing manual Grimoire install can migrate. Unrelated documents are never replaced.
        if ! head -n 10 "$DOCS/$name" | grep -q '^# Author: Ganten$' ||
           ! head -n 10 "$DOCS/$name" | grep -q '^# Name: Grimoire'; then
            echo "Existing $name is not a Grimoire launcher; keeping it." >&2
            exit 1
        fi
        cp "$DOCS/$name" "$WORK/old/$name"
    fi
    cp "$DIR/scriptlets/$name" "$STAGE-$name"
    chmod 755 "$STAGE-$name"
done
# All staging succeeded before either installed launcher changes.
changed=yes
for name in Grimoire.sh 'Grimoire - reset pen calibration.sh'; do
    mv -f "$STAGE-$name" "$DOCS/$name"
done
ok=yes
echo 'Grimoire installed. Notebooks were kept; no KOReader plugin or patches were installed.'
