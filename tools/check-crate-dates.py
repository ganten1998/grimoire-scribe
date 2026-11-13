#!/usr/bin/env python3 -I
"""The crate-date check of docs/RELEASING.md, step 3 (DESIGN.md 5, D22): every crate in rust/Cargo.lock must have
been published on crates.io at least two weeks before the tag. Prints each crate's date, then the ones too young
with the newest version that is old enough, and exits 1 if there are any.

usage: tools/check-crate-dates.py [--tag-date YYYY-MM-DD] [--lock rust/Cargo.lock]

--tag-date is the day the release will be tagged (default: today). The two weeks count back from it, so a
lock checked a week before the tag passes only with the tag's own date. Needs the network (crates.io's API, one
request a second, as its crawler policy asks).
"""
import datetime
import json
import re
import sys
import time
import urllib.request

AGE_DAYS = 14


def crates(lock):
    out = []
    for block in re.split(r"\n(?=\[\[package\]\])", open(lock, encoding="utf-8").read())[1:]:
        n = re.search(r'^name = "(.*?)"', block, re.M).group(1)
        v = re.search(r'^version = "(.*?)"', block, re.M).group(1)
        src = re.search(r'^source = "(.*?)"', block, re.M)
        if src and src.group(1).startswith("registry+"):
            out.append((n, v))
    return out


def api(name):
    req = urllib.request.Request(f"https://crates.io/api/v1/crates/{name}", headers={"User-Agent": "grimoire-release-check"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)


def main():
    tag = datetime.date.today()
    lock = "rust/Cargo.lock"
    a = sys.argv[1:]
    while a:
        k = a.pop(0)
        if k == "--tag-date":
            tag = datetime.date.fromisoformat(a.pop(0))
        elif k == "--lock":
            lock = a.pop(0)
        else:
            print(__doc__)
            return 2
    cutoff = tag - datetime.timedelta(days=AGE_DAYS)
    young = []
    for name, ver in sorted(crates(lock)):
        d = api(name)
        vs = {v["num"]: v["created_at"][:10] for v in d["versions"] if not v["yanked"]}
        made = datetime.date.fromisoformat(vs[ver]) if ver in vs else None
        print(f"{name} {ver} {made}")
        if made is None or made > cutoff:
            older = [(vs[n], n) for n in vs if datetime.date.fromisoformat(vs[n]) <= cutoff and n.split(".")[0] == ver.split(".")[0]]
            best = max(older)[1] if older else "none"
            young.append((name, ver, made, best))
        time.sleep(1)
    if young:
        print(f"\nToo young for a tag on {tag} (published after {cutoff}):")
        for name, ver, made, best in young:
            print(f"  {name} {ver} ({made}); newest old enough: {best}:  (cd rust && cargo update -p {name}@{ver} --precise {best})")
        return 1
    print(f"\nEvery crate is at least {AGE_DAYS} days old on {tag}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
