#!/bin/sh
# Finish a Grimoire widget package: put a built artifact at the manifest's artifact.path and write
# the size and SHA-256 the installer checks. With no built file, check the pair already there.
#
# usage: tools/widget-pack.sh PACKAGE_DIR [BUILT_FILE]
#
# PACKAGE_DIR is the folder with package.yaml (the repository root). BUILT_FILE is the single file
# the package installs (a .wasm module, or the calculator's placeholder). It is copied to
# artifact.path, then artifact.size and artifact.sha256 are rewritten. Without BUILT_FILE the
# existing artifact must already match.
#
# The manifest must use the block form of artifact: with path, sha256 and size. This script edits
# only those two numbers: bump version yourself, and update a releases entry for the current
# version by hand (it has its own sha256).

set -u

die() {
    printf 'widget-pack: %s\n' "$*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
usage: tools/widget-pack.sh PACKAGE_DIR [BUILT_FILE]

  PACKAGE_DIR  the folder holding package.yaml
  BUILT_FILE   copy this file to artifact.path and write its size and SHA-256

Without BUILT_FILE, check the artifact already at artifact.path against the manifest.
EOF
}

case "${1:-}" in
    -h|--help)
        usage
        exit 0
        ;;
esac

[ $# -ge 1 ] && [ $# -le 2 ] || {
    usage >&2
    exit 2
}

dir=$1
built=${2:-}
manifest=$dir/package.yaml

[ -d "$dir" ] || die "not a directory: $dir"
[ -f "$manifest" ] || die "no package.yaml in $dir"
[ -r "$manifest" ] || die "cannot read $manifest"

if grep -Eq '^artifact:[[:blank:]]*\{' "$manifest"; then
    die "package.yaml uses an inline artifact: mapping; write it as a block with path, sha256 and size"
fi
if ! grep -Eq '^artifact:[[:blank:]]*$' "$manifest"; then
    die "package.yaml has no top-level artifact: block"
fi

# A scalar from package.yaml's artifact: block, unquoted.
artifact_field() {
    awk -v key="$1" '
        BEGIN { inart = 0 }
        /^artifact:[[:blank:]]*$/ { inart = 1; next }
        inart && /^[^[:blank:]]/ { exit }
        inart {
            line = $0
            sub(/^[[:blank:]]*/, "", line)
            name = line
            sub(/:.*/, "", name)
            if (name != key) next
            sub(/^[^:]*:[[:blank:]]*/, "", line)
            sub(/[[:blank:]]+#.*$/, "", line)
            sub(/[[:blank:]]+$/, "", line)
            if (line ~ /^".*"$/ || line ~ /^\047.*\047$/)
                line = substr(line, 2, length(line) - 2)
            print line
            exit
        }
    ' "$manifest"
}

path=$(artifact_field path)
sha=$(artifact_field sha256)
size=$(artifact_field size)

[ -n "$path" ] || die "artifact.path is missing"
[ -n "$sha" ] || die "artifact.sha256 is missing (add a 64-digit placeholder first)"
[ -n "$size" ] || die "artifact.size is missing (add a placeholder first)"
[ ${#path} -le 240 ] || die "artifact.path is longer than 240 characters"

case $path in
    /*) die "artifact.path must be relative" ;;
    package.yaml) die "artifact.path cannot be package.yaml" ;;
esac
case $path in
    *\\*) die "artifact.path cannot contain a backslash" ;;
esac
case "/$path/" in
    *"//"*) die "artifact.path has an empty component" ;;
    *"/../"*|*"/./"*) die "artifact.path has a . or .. component" ;;
esac

dest=$dir/$path

if [ -n "$built" ]; then
    [ -f "$built" ] || die "not a file: $built"
    [ -r "$built" ] || die "cannot read $built"
    mkdir -p "$(dirname "$dest")" || die "cannot make the folder for $dest"
    if [ -e "$dest" ] && cmp -s "$built" "$dest"; then
        :
    else
        cp "$built" "$dest" || die "cannot copy $built to $dest"
    fi
fi

[ -f "$dest" ] || die "no artifact at $dest (pass the built file to install it)"

sha_tool=
if command -v sha256sum >/dev/null 2>&1; then
    sha_tool=sha256sum
elif command -v shasum >/dev/null 2>&1; then
    sha_tool=shasum
elif command -v openssl >/dev/null 2>&1; then
    sha_tool=openssl
else
    die "need sha256sum, shasum or openssl to compute the digest"
fi

if [ "$sha_tool" = sha256sum ]; then
    got_sha=$(sha256sum "$dest" | awk '{ print $1 }') || die "sha256sum failed"
elif [ "$sha_tool" = shasum ]; then
    got_sha=$(shasum -a 256 "$dest" | awk '{ print $1 }') || die "shasum failed"
else
    got_sha=$(openssl dgst -sha256 "$dest" | sed 's/.*= //') || die "openssl failed"
fi
got_sha=$(printf '%s' "$got_sha" | tr 'ABCDEF' 'abcdef')
case $got_sha in
    ""|*[!0-9a-f]*) die "could not read the artifact's SHA-256" ;;
esac
[ ${#got_sha} -eq 64 ] || die "could not read the artifact's SHA-256"

got_size=$(wc -c < "$dest" | tr -d '[:space:]')
[ -n "$got_size" ] || die "could not read the artifact's size"
[ "$got_size" -gt 0 ] 2>/dev/null || die "the artifact is empty"
[ "$got_size" -le 33554432 ] 2>/dev/null || die "the artifact is over 32 MiB"

if [ -z "$built" ]; then
    old_sha=$(printf '%s' "$sha" | tr 'ABCDEF' 'abcdef')
    if [ "$old_sha" = "$got_sha" ] && [ "$size" = "$got_size" ]; then
        printf 'widget-pack: %s is up to date (%s bytes, sha256 %s)\n' "$path" "$got_size" "$got_sha"
        exit 0
    fi
    printf 'widget-pack: %s does not match package.yaml\n' "$path" >&2
    printf '  manifest: %s bytes, sha256 %s\n' "$size" "$sha" >&2
    printf '  artifact: %s bytes, sha256 %s\n' "$got_size" "$got_sha" >&2
    printf 'widget-pack: pass the built file to update the manifest\n' >&2
    exit 1
fi

tmp=$manifest.tmp.$$
trap 'rm -f "$tmp"' 0
trap 'exit 1' 1 2 15

if ! awk -v size="$got_size" -v sha="$got_sha" '
    function indent(s) {
        match(s, /^[[:blank:]]*/)
        return substr(s, 1, RLENGTH)
    }
    BEGIN { inart = 0; done_size = 0; done_sha = 0 }
    /^artifact:[[:blank:]]*$/ { inart = 1; print; next }
    inart && /^[^[:blank:]]/ { inart = 0 }
    {
        if (inart) {
            line = $0
            sub(/^[[:blank:]]*/, "", line)
            name = line
            sub(/:.*/, "", name)
            if (name == "size") {
                print indent($0) "size: " size
                done_size = 1
                next
            }
            if (name == "sha256") {
                print indent($0) "sha256: " sha
                done_sha = 1
                next
            }
        }
        print
    }
    END {
        if (!done_size || !done_sha)
            exit 3
    }
' "$manifest" > "$tmp"; then
    die "package.yaml has no artifact.size or artifact.sha256 line to replace"
fi

mv "$tmp" "$manifest" || die "cannot write $manifest"
tmp=
printf 'widget-pack: %s -> %s (%s bytes, sha256 %s)\n' "$built" "$path" "$got_size" "$got_sha"
printf 'widget-pack: package.yaml updated; bump version and update any releases entry by hand\n'
