#!/usr/bin/env python3 -I
"""app/licenses/rust-crates.txt (2.1.0-beta.2, DESIGN.md 5, D22): the licenses of the Rust crates Cargo.lock
names for the Kindle binary (all of them: built in, build time only, or named and not compiled), and the notices
of the data hayro carries (Foxit's standard fonts, Adobe's CMaps, the ICC profiles). Written at package time from
`cargo metadata --frozen` for the Kindle target, so it names exactly the crates Cargo.lock holds;
tests/package_check.sh checks every one of them is in it.

A crate offered under "A OR B" is taken under the first of MIT, Zlib, BSD-3-Clause, Unlicense, 0BSD and
Apache-2.0 it offers (the shortest text; the copyright lines stay), as the licenses allow a recipient to choose;
a crate under "A AND B" gets both. Never run with a data root that is not the repository: it only reads.

usage: crate_licenses.py <rust dir> <out file> [cargo]
"""
import json
import os
import re
import subprocess
import sys

PREFER = ["MIT", "Zlib", "BSD-3-Clause", "Unlicense", "0BSD", "Apache-2.0"]
# which license file names hold which license (the crates' own habits)
FILES = {
    "MIT": ["LICENSE-MIT", "LICENSE-MIT.md", "LICENSE.MIT", "LICENSE-MIT.txt", "LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING"],
    "Zlib": ["LICENSE-ZLIB", "LICENSE-ZLIB.md", "LICENSE", "LICENSE.md"],
    "BSD-3-Clause": ["LICENSE.BSD-3-Clause", "LICENSE.md", "LICENSE"],
    "Unlicense": ["UNLICENSE", "LICENSE-UNLICENSE", "COPYING"],
    "0BSD": ["LICENSE-0BSD", "LICENSE"],
    "Apache-2.0": ["LICENSE-APACHE", "LICENSE-APACHE.md", "LICENSE.APACHE", "LICENSE"],
    "Unicode-3.0": ["LICENSE-UNICODE", "LICENSE"],
}


def read(p):
    with open(p, encoding="utf-8", errors="replace") as f:
        return f.read().strip()


# a crate that ships no license file takes its sibling's (the same authors, the same text)
SIBLING = {"alloc-stdlib": "alloc-no-stdlib"}


def pick(expr):
    """The licenses to include for an SPDX expression (a crate's own text of it)."""
    expr = expr.replace("/", " OR ").replace("(", " ").replace(")", " ")
    out = []
    for conj in re.split(r"\s+AND\s+", expr):
        opts = [o.strip() for o in re.split(r"\s+OR\s+", conj) if o.strip()]
        best = next((p for p in PREFER if p in opts), opts[0] if opts else None)
        if best and best not in out:
            out.append(best)
    return out


def text_of(d, lic):
    names = FILES.get(lic, ["LICENSE"]) + ["LICENSE", "LICENSE.md", "COPYING", "LICENSE.txt"]
    for n in names:
        p = os.path.join(d, n)
        if os.path.isfile(p):
            t = read(p)
            # a file for the wrong license (a dual crate's LICENSE that is the other one) is still the
            # crate's own text; the first match in order is taken
            return n, t
    return None, None


def main():
    rust, out = sys.argv[1], sys.argv[2]
    cargo = sys.argv[3] if len(sys.argv) > 3 else "cargo"
    meta = json.loads(
        subprocess.run(
            [cargo, "metadata", "--format-version", "1", "--frozen", "--filter-platform", "armv7-unknown-linux-musleabihf"],
            cwd=rust, check=True, capture_output=True, text=True,
        ).stdout
    )
    pk = sorted((p for p in meta["packages"] if p["name"] != "grimoire"), key=lambda p: (p["name"], p["version"]))
    lines = [
        "From the Rust crates Cargo.lock names for the Grimoire binary. Every one is listed: those it is built",
        "from, the few that only run while it is built, and the few the lock names that are not compiled in. Each",
        "was at least two weeks old at its release (docs/RELEASING.md). Each is listed with its version, the",
        "license it is used under and that license's text as the crate ships it. The hayro crates (PDF reading)",
        "also carry data with licenses of its own, which follow the crates.",
        "",
    ]
    for p in pk:
        d = os.path.dirname(p["manifest_path"])
        lic = p.get("license") or "see the crate"
        lines.append("=" * 78)
        lines.append(f"{p['name']} {p['version']}  (licensed {lic})")
        lines.append(f"{p.get('repository') or p.get('homepage') or 'https://crates.io/crates/' + p['name']}")
        lines.append("=" * 78)
        seen = set()
        for l in pick(lic) if p.get("license") else ["MIT"]:
            n, t = text_of(d, l)
            if t and t not in seen:
                seen.add(t)
                lines += ["", f"[{l}: {n}]", "", t, ""]
        if not seen and p["name"] in SIBLING:
            q = next((q for q in pk if q["name"] == SIBLING[p["name"]]), None)
            if q:
                n, t = text_of(os.path.dirname(q["manifest_path"]), "BSD-3-Clause")
                if t:
                    seen.add(t)
                    lines += ["", f"[{lic}: the text of {q['name']}, whose authors these are too]", "", t, ""]
        if not seen:
            lines += ["", "(this crate ships no license file; its manifest says: " + lic + ")", ""]
    # the data hayro carries
    extra = [
        ("Foxit's standard fonts (hayro-interpret, the embed-fonts feature)", "hayro-interpret", ["assets/LICENSE_FOXIT"]),
        ("Adobe's CMaps (hayro-cmap, the embed-cmaps feature)", "hayro-cmap", ["assets/LICENSE.txt"]),
        ("the ICC profiles (hayro-interpret)", "hayro-interpret", ["assets/CGATS_LICENSE.txt"]),
    ]
    for title, crate, files in extra:
        p = next((q for q in pk if q["name"] == crate), None)
        if not p:
            continue
        d = os.path.dirname(p["manifest_path"])
        lines += ["", "=" * 78, title, "=" * 78, ""]
        for f in files:
            fp = os.path.join(d, f)
            lines.append(read(fp) if os.path.isfile(fp) else f"(missing: {f})")
            lines.append("")
    with open(out, "w", encoding="utf-8") as f:
        f.write("\n".join(lines).rstrip() + "\n")


if __name__ == "__main__":
    main()
