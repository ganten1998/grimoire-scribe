# Releasing Grimoire

How a release is cut: the version bump, the gate, the dry run in CI, the tag, the GitHub release,
the KPM feed that Settings > Updates reads, and Codeberg. The KPM side (package, feeds, hotfixes) is
in [KPM.md](KPM.md); what was tried on a Kindle is in [TESTING.md](TESTING.md).

Nothing here happens by itself. The workflow (`.github/workflows/release.yml`) checks and builds on
every pull request and manual run, and **publishes only when a GitHub release is published**: it
then uploads the release's files and writes the KPM feed. Everything before that is done by hand,
on the Mac, after the owner has tried the build on a Kindle. Never bump the version, tag, push a
tag or publish from a work-in-progress branch.

## What a release is

| | |
|---|---|
| Version | `VERSION`: `MAJOR.MINOR.PATCH`, with `-beta.N` or `-rc.N` (N from 1 to 999) for a pre-release |
| Tag | `vVERSION`, annotated, on the commit the last gate passed on, whose binary the owner tested |
| GitHub release | title `Grimoire 2.0 beta 5` (a stable one: `Grimoire 1.2.0`); a beta or RC is a **pre-release** and not Latest; only a stable release is Latest |
| Assets | `Grimoire-VERSION.zip`, `grimoire_KPM_kindlehf.kpkg`, `KOReader-patches.zip`, `SHA256SUMS.txt` |
| KPM number | `[major, minor, patch*10000+stage]`: beta.N = 1000+N, rc.N = 5000+N, final = 9999. 2.0.0-beta.5 is 2.0.1005, 2.0.0 is 2.0.9999 |
| Feed | the `kpm` branch: `preview/` for any release (a stable one too), `stable/` for a stable one. A feed never moves to an older version |
| Codeberg | the branch and the tag are pushed; no release page or assets so far (there is no API token on the Mac) |

The bytes are reproducible: the same commit, with the same rustc, zig and cargo-zigbuild, makes the
same zip, patches zip and package on any Mac and in the workflow, and `tests/repro_check.sh` (part
of `make check`) builds twice from clean to hold it to that. So the binary the owner tested is the
binary that ships, and the files CI builds are the files the Mac builds: the dry run below compares
them.

## Before the tag

1. **One branch, one commit.** Merge everything the release holds. Betas have been tagged on the
   branch that built them while `main` stayed on the last stable release (v2.0.0-beta.3), and once
   `main` was moved to the tag (v2.0.0-beta.1); which one is the owner's call. The workflow checks
   out the tag, so either works. A stable release goes to `main` first, on both remotes.

2. **The toolchain is the workflow's.** The three versions at the top of the workflow
   (`RUST_VERSION`, `ZIG_VERSION`, `CARGO_ZIGBUILD_VERSION`) must be what the Mac builds with, or
   the zip the owner tested differs from the one CI builds. The Makefile's default `RUST_BIN` is
   rustup's `stable`, which moves: pass the pinned toolchain to every make of the release.

   ```sh
   export RUST_BIN=$HOME/.rustup/toolchains/1.97.1-aarch64-apple-darwin/bin
   $RUST_BIN/rustc --version      # rustc 1.97.1 (...)
   zig version                    # 0.16.0
   cargo-zigbuild --version       # cargo-zigbuild 0.23.0
   ```

3. **Every crate is at least two weeks old.** `rust/Cargo.lock` decides every crate the build
   downloads, and the policy for everything the workflow downloads is a release at least two weeks
   old. Look at the dates before the tag:

   ```sh
   cd rust && for c in $(awk '/^name = / { gsub(/"/, "", $3); n = $3 } /^version = / { gsub(/"/, "", $3); if (n != "" && n != "grimoire") print n "/" $3 }' Cargo.lock); do
     curl -s -A 'grimoire-release' "https://crates.io/api/v1/crates/$c" | python3 -I -c 'import json, sys; v = json.load(sys.stdin)["version"]; print(v["crate"], v["num"], v["created_at"][:10])'
   done
   ```

   A crate that is younger goes back to its newest version older than that: `cargo update -p CRATE
   --precise VERSION` in `rust/`, then the gate again. A crate is only ever moved to a version that
   is old enough.

4. **The bump commit.** One commit, `release: prepare VERSION`, with:
   - `VERSION`. Everything else reads it: the binary (build.rs), the zip's name and README.txt, the
     KPM package, manifest and feed, `dist/`. No code holds a copy of it.
   - `CHANGELOG.md`: the section's `(unreleased)` becomes the date, `(2026-10-30)`; every
     `RELEASE-PLACEHOLDER` line is replaced by its entry in plain words, or deleted. A section of an
     earlier candidate that was never published is marked `(candidate, folded into VERSION)`.
   - `README.md`: the banner, `Pre-release VERSION.` (a stable release has no banner).
   - `docs/TESTING.md`: a `## VERSION` section, with what the owner checked on a Kindle and what
     still needs a device. Claim only what the owner reported.
   - `po/grimoire.pot` carries the version in its header: `make l10n` (needs GNU gettext) rewrites
     it, and the .po and .mo files with it. A stale header does no harm.

   Then `tools/release-preflight.sh` checks the version, the changelog's dated heading, the
   placeholders, the README banner, the TESTING section and a clean tree. The workflow runs it again
   on the release's tag, first thing.

5. **The gate on the bump commit.** The version is in the binary, so the gate runs after the bump,
   with the pinned toolchain and no skipped part:

   ```sh
   make check DIST=$HOME/Library/Caches/grimoire-dist
   cp build/kindle/grimoire $HOME/Library/Caches/grimoire-dist/tested-grimoire
   ```

   `make check` is `make rust-check` plus the zip and its launchers, the clean-build comparison, the
   KPM package and the KOReader patches, and ends by collecting `Grimoire-VERSION-COMMIT/` in
   `DIST` (never in Downloads): the zip, the patches zip, the `.kpkg`, the two manifests,
   `SOURCE.json` and a `SHA256SUMS.txt` of all of them.
   Read the log for `skip` lines. Set `GRIMOIRE_TEST_PORT` when another session uses the share
   server's port.

6. **The owner's test.** Install that bundle on the Kindle (README, Install; or KPM, docs/KPM.md).
   The owner's results go into `docs/TESTING.md` as a docs-only commit on top of the bump commit,
   and nothing else may follow the test: a fix is a new commit, a new gate and a new test. A commit
   changes the zip (its files carry the commit's time, and `BUILD_ID` is the checksum of what holds
   them) but not the binary, so after the docs commit run `make check` again on it and compare:

   ```sh
   cmp build/kindle/grimoire $HOME/Library/Caches/grimoire-dist/tested-grimoire && echo "the tested binary"
   tools/release-preflight.sh
   ```

   The commit that is tagged is that docs commit. Its dist bundle, not the tested one, is what the
   dry run below compares with CI.

## The dry run in CI

Push the branch to GitHub and start the workflow on it. It builds, checks and uploads the
`grimoire-release` artifact, and publishes nothing (only a published release does):

```sh
git push github BRANCH
gh workflow run release.yml --ref BRANCH -R ganten1998/grimoire-scribe
gh run list -R ganten1998/grimoire-scribe --workflow release.yml --limit 3
gh run watch RUN -R ganten1998/grimoire-scribe --exit-status
```

(A draft pull request into `main` runs it too, on the pull request's commit.) Then compare what CI
built with what the owner tested, byte for byte:

```sh
rm -rf /tmp/grimoire-ci && gh run download RUN -R ganten1998/grimoire-scribe -n grimoire-release -D /tmp/grimoire-ci
D=$HOME/Library/Caches/grimoire-dist/Grimoire-VERSION-COMMIT
for f in Grimoire-VERSION.zip KOReader-patches.zip grimoire_KPM_kindlehf.kpkg manifest.json manifest.v2.json; do
  cmp /tmp/grimoire-ci/$f $D/$f && echo "same  $f"
done
```

All five must be the same. If one is not, **do not release**: the owner tested other bytes than CI
would publish. The usual causes are a different rustc, zig or cargo-zigbuild version, a different
crate in `Cargo.lock`, or a path or environment that reached the binary (tests/repro_check.sh
looks for the last kind). Fix the cause, gate, test again.

The Rust workflow was written on a Mac and had not run on a GitHub runner when this was written.
The first dry run is where its mistakes show: the toolchain steps, the time (the C workflow took
about 28 minutes, `timeout-minutes` is 120), the disk on the runner. Delete this paragraph once it
has passed.

## Tag and publish

Only now, and only when the owner says so. Both remotes get the branch and the tag; GitHub's
release is what publishes.

```sh
git tag -a vVERSION -m "Grimoire VERSION" COMMIT
git push github BRANCH vVERSION
GIT_SSH_COMMAND='ssh -o IdentityAgent=none -o BatchMode=yes' git push codeberg BRANCH vVERSION
```

(`IdentityAgent=none` is for a shell without the login agent, as Claude Code's; in the owner's own
terminal plain `git push codeberg ...` works.) Write the release notes as `notes.md` in the style of
the earlier releases (`gh release view v2.0.0-beta.3`): one sentence on what the release is, the
user-visible changes in short bullets from the CHANGELOG, how to install it, which feed and which
Latest it does or does not move, and one line of what the owner checked on a Kindle, with its
firmware. Then:

```sh
gh release create vVERSION -R ganten1998/grimoire-scribe --verify-tag --title "Grimoire 2.0 beta 5" \
  --notes-file notes.md --prerelease --latest=false
```

A stable release drops `--prerelease --latest=false`. The workflow's `release: published` run
starts at once: it checks the tag against `VERSION` and the pre-release flag against the version's
suffix, runs the preflight and the whole gate, builds the files, uploads them to the release,
and, once they are there, writes the feed. It never replaces an existing asset: a file already
on the release (uploaded earlier from verified bytes) must be byte-identical or the job fails.

```sh
gh run watch $(gh run list -R ganten1998/grimoire-scribe --workflow release.yml --limit 1 --json databaseId --jq '.[0].databaseId') \
  -R ganten1998/grimoire-scribe --exit-status
```

### Codeberg

The push to Codeberg above is all it gets by default. A release page there is optional
and is made in the web UI (Releases > New release, choose the tag, paste the same notes, attach
the four files from the GitHub release), because no API token is stored on the Mac.

## The feed, and the updater

The workflow's publish job writes the feed: it checks out the `kpm` branch, runs
`tools/kpm-feed.py build/kpm/manifest.json FEED`, and pushes the commit
`Publish Grimoire vVERSION to KPM`. A beta or RC changes `preview/` only; a stable release changes
`stable/` and also `preview/`. Settings > Updates on a Kindle then offers it: Grimoire adds its
channel's feed to KPM, runs `kpm update` and reads the newest `grimoire` in `kpm search`, whose
description ends with the Grimoire version (kindle/app/update.sh, tools/kpm-package.py). That needs,
all three, what the publish job guarantees: the release's `.kpkg` is downloadable at the URL in the
manifest, its sha256 is the manifest's, and its KPM number is higher than the installed one.

The updater first exists in 2.0.0-beta.5. A Kindle on an older version has no Updates sheet: it
gets beta.5 from the zip, or from KPM after the preview feed is added by hand (docs/KPM.md,
Updating); from beta.5 on, the sheet offers each later release.

Check the result a few minutes after the run (raw.githubusercontent.com caches briefly):

```sh
curl -s https://raw.githubusercontent.com/ganten1998/grimoire-scribe/kpm/preview/manifest.json | python3 -I -c \
  'import json, sys; a = json.load(sys.stdin)["packages"]["grimoire"]["artifacts"][0]; print(a["grimoire_version"], a["version"], a["url"], a["sha256"])'
gh release download vVERSION -R ganten1998/grimoire-scribe -D /tmp/grimoire-rel --clobber
(cd /tmp/grimoire-rel && shasum -a 256 -c SHA256SUMS.txt)
```

The printed version, KPM number and URL are the release's, and its `.kpkg`'s sha256 is the
manifest's. If the publish job could not write the feed (the assets are uploaded, the push
failed), do what it does by hand. The manifest is the one of the gate's bundle (`$D` above), the
same bytes as the workflow's:

```sh
git fetch github kpm && git worktree add /tmp/grimoire-feed FETCH_HEAD
python3 tools/kpm-feed.py $D/manifest.json /tmp/grimoire-feed
cd /tmp/grimoire-feed && git add -A && git commit -m "Publish Grimoire vVERSION to KPM preview" && git push github HEAD:refs/heads/kpm
cd - && git worktree remove --force /tmp/grimoire-feed
```

## When something fails

- **The run failed on the runner (network, a download, time).** Re-run the failed jobs
  (`gh run rerun RUN --failed`): the commit is the same, so the bytes are. The publish job can be
  re-run on its own and is safe to: it uploads only what is missing and checks the rest.
- **The run failed because of the tree** (a check, the preflight, a pin). The release was cut too
  early. If the feed was not written (check the `kpm` branch), delete the release and the tag
  (`gh release delete vVERSION --cleanup-tag -y`, then `git push codeberg :refs/tags/vVERSION`), fix
  the tree, gate, test, and tag again. If the feed was written, the version is spent: cut the next.
- **An existing asset differs from the new bytes.** The job fails on purpose. A published version is
  never reused: KPM compares only the numbers, so a rebuilt package under the same version is never
  offered as an update. Cut the next version.
- **A bad release is in the feed.** The next version replaces it for everyone at their next Check for
  updates. To take it out sooner, revert the feed's commit on the `kpm` branch (plain git; the
  channel then offers the version before it). Notebooks are never touched by an install either way.
- **A tag was pushed too early.** Before the release is published, a tag can be moved or deleted;
  the workflow only runs on a published release.

## What the workflow pins, and how to move a pin

Every download is a pinned version at least two weeks old when it is first used. To move one, look
up its date first and write the new version only if it is old enough; the Mac and the workflow
change together, and the dry run above compares them.

| What | Where it is pinned | Checked by |
|---|---|---|
| rustc, cargo, the Kindle's standard library | `RUST_VERSION` (rustup; 1.97.1 was built 2026-07-14) | rustup's own signed manifest; the job fails if `rustc --version` differs |
| zig | `ZIG_VERSION` (0.16.0, 2026-04-13) and its sha256 in the "Install zig" step | `shasum -c` against the hash written in the workflow, not one that came with the download |
| cargo-zigbuild | `CARGO_ZIGBUILD_VERSION` (0.23.0, 2026-06-16), `cargo install --locked` | crates.io's checksums and the crate's own lockfile; the job fails if `--version` differs |
| the app's crates | `rust/Cargo.lock` (`cargo fetch --locked` fails on a lockfile that would change) | crates.io's checksums |
| actions | commit SHAs: `actions/checkout` v4.4.0, `setup-python` v5.6.0, `upload-artifact` v4.6.2, `download-artifact` v4.3.0 | the SHA |
| the vendored C | `third_party/`, in the tree | git |

Not pinned: the Homebrew test tools (gettext, luajit, ImageMagick, ShellCheck, luacheck) and the
runner image's Node and Xcode. They run checks and the host build; none of them makes a byte of a
release file (the Kindle binary is rustc and zig, the zips and the package are Python and the tree).
To pin a zig sha256 for a new version: `curl -s https://ziglang.org/download/index.json`, the
`aarch64-macos` and `x86_64-macos` entries of that version.

## Hotfixes

A hotfix is the next version on its channel through this same process: docs/KPM.md, Hotfixes.
