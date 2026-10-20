#!/usr/bin/env python3
# l10n_markup.py: a catalog whose every translation carries markup, and the check that the share
# pages show it as text. A catalog is third-party input: none may add markup to a page.
#   l10n_markup.py make IN.mo OUT.mo   IN with MARK (tags, quotes, an entity) before each of its
#                                      translations (each plural form); the header stays
#   l10n_markup.py check PAGE.html [--script]
#                                      PAGE holds MARK escaped (html_esc, httpd.c), and with
#                                      --script also as json_str writes it inside a <script>, and
#                                      never MARK's markup itself
# tests/share_check.sh serves the pages from such a catalog (the simulator's GRIMOIRE_L10N_DIR).
import struct
import sys

MARK = "</script><x-l10n a='\"'>&amp;"
HTML = MARK.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace('"', "&quot;").replace("'", "&#39;")
JSON = MARK.replace('"', '\\"').replace("<", "\\u003c").replace(">", "\\u003e").replace("&", "\\u0026")


def make(src, dst):
    d = open(src, "rb").read()
    e = "<" if struct.unpack("<I", d[:4])[0] == 0x950412DE else ">"
    n, orig_at, trans_at = struct.unpack(e + "3I", d[8:20])

    def table(at):
        out = []
        for i in range(n):
            ln, off = struct.unpack(e + "2I", d[at + 8 * i : at + 8 * i + 8])
            out.append(d[off : off + ln])
        return out

    orig, trans = table(orig_at), table(trans_at)
    mark = MARK.encode()
    trans = [t if not o or not t else b"\0".join(mark + f for f in t.split(b"\0")) for o, t in zip(orig, trans)]
    # The same English in the same order (sorted, as msgfmt writes it), no hash table.
    start = 28 + 16 * n
    blob, tabs = b"", [b"", b""]
    for k, strings in enumerate((orig, trans)):
        for s in strings:
            tabs[k] += struct.pack("<2I", len(s), start + len(blob))
            blob += s + b"\0"
    head = struct.pack("<7I", 0x950412DE, 0, n, 28, 28 + 8 * n, 0, start)
    open(dst, "wb").write(head + tabs[0] + tabs[1] + blob)


def check(page, script):
    s = open(page, encoding="utf-8").read()
    bad = []
    if "<x-l10n" in s:
        at = s.index("<x-l10n")
        bad.append("markup from the catalog as markup: ..." + s[max(0, at - 60) : at + 40] + "...")
    if HTML not in s:
        bad.append("no translation shown escaped (" + HTML + ")")
    if script and JSON not in s:
        bad.append("no translation in the script escaped (" + JSON + ")")
    for b in bad:
        print(page + ": " + b)
    return not bad


if __name__ == "__main__":
    a = sys.argv[1:]
    if len(a) == 3 and a[0] == "make":
        make(a[1], a[2])
    elif len(a) in (2, 3) and a[0] == "check" and (len(a) == 2 or a[2] == "--script"):
        sys.exit(0 if check(a[1], len(a) == 3) else 1)
    else:
        sys.exit("usage: l10n_markup.py make IN.mo OUT.mo | check PAGE.html [--script]")
