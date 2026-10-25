# KPM installation and release pipeline

Grimoire packages the tested self-contained ZIP launcher as a KPM v2 `.kpkg`. The initial target
is the **Kindle Scribe on kindlehf**. KPM's platform field cannot restrict the package to Scribe
models; other kindlehf Kindles do not gain pen support by installing it.

Download the `.kpkg` from the release and copy it to the Kindle. From an existing Kindle shell:

```sh
/var/local/kmc/bin/kpm -y install file:///mnt/us/Downloads/grimoire_2.0.1001_kindlehf.kpkg
/var/local/kmc/bin/kpm launch grimoire
```

Replace the filename with the asset actually downloaded. The Library launchers call `kpm launch
grimoire`; the complete app stays in KPM's package directory and unpacks into `grimoire/app` on
first launch. A reset-calibration Library item is included. Close Grimoire before installing,
upgrading or removing it. Quit KOReader before a normal Library launch.

Installation can migrate the two official manual Grimoire launchers. It stages both beside their destinations before
replacing either (the Kindle blocks cross-directory moves into Documents), restores them if a hook fails, and refuses unrelated documents with those
names. An existing running Grimoire blocks the hook. KPM itself controls package extraction and
upgrade rollback; the hook cannot make KPM's own package replacement transactional.
Temporary installer bookkeeping lives in `/tmp`, leaving an older `quire/` data directory
eligible for the payload's normal first-launch migration instead of creating an empty `grimoire/`.

Uninstall with `kpm -y uninstall grimoire`. Only unchanged KPM Library launchers are removed.
**All `grimoire/` data remains**, including notebooks, settings, PDFs, mappings and unpack cache.
That cache may still serve an explicitly installed KOReader plugin or manual installation.
Remove the data folder yourself only when you intend to delete it. KPM installation/removal never
installs or removes KOReader plugins, patches, fonts, PIN flags or root filesystem files.

## Updating

Once a release has been published through the workflow (Build and publication, below), add the
feed of the channel you want, then install or update Grimoire by name. Until the first stable
release, only the preview feed exists. On the Kindle, the search bar did not accept the punctuation
of a feed URL, so add the feed from a shell on the Kindle (over SSH, for example) with the `kpm`
command:

```sh
/var/local/kmc/bin/kpm add-repo https://raw.githubusercontent.com/ganten1998/grimoire-scribe/kpm/preview/manifest.json
/var/local/kmc/bin/kpm -y install grimoire
```

Updates from a feed run Grimoire's upgrade hooks. They were tested with KPM 0.2.2, from local test
feeds (1.1.0-beta.2 to 1.1.0-rc.1, then 1.1.0-rc.1 to 1.1.0-rc.2), and 1.1.0-rc.3 was installed
with KPM 0.2.4. 2.0.0-beta.1 (KPM 2.0.1001) sorts after those candidates (KPM 1.1.5001 to
1.1.5003).

**KPM 0.2.2 local-file limitation:** installing a second `file://...kpkg` over an already installed
package can fail in KPM's dependency resolver before Grimoire's hooks run. Its `upgrade` command
also skips locally installed packages. This was seen with KPM 0.2.2 and has not been checked again
with 0.2.4. For a manual-file update, download the new package first, close Grimoire, then use:

```sh
/var/local/kmc/bin/kpm -y uninstall grimoire
/var/local/kmc/bin/kpm -y install file:///mnt/us/Downloads/grimoire_2.0.1001_kindlehf.kpkg
```

The hooks retain `grimoire/` through both commands. This uninstall/reinstall fallback was tested
against the same notebook/settings sentinels left by the beta app. A normal repository update
uses KPM's upgrade hooks and does not need that workaround.

## Build and publication

`make kpm-package` builds the ZIP first, then `tools/kpm-package.py` produces the deterministic
package, repository manifests and checksum in `build/kpm`. `make check` includes KPM tests.

Merge the workflow onto the default branch before publishing the first release through it.
The GitHub release workflow runs the full checks. Only a **published GitHub release** uploads
the ZIP, optional patches, KPM package and checksums, then updates the repository's `kpm` branch.
Pull requests, pushes and manual workflow runs validate/build without publishing. Tag and VERSION
must match, and GitHub prerelease status must match the version suffix.

The first published release through this workflow, 2.0.0-beta.1, creates the preview feed: a beta
or RC writes only that one. The stable feed appears with the first stable release, 2.0.0:

- Preview: `https://raw.githubusercontent.com/ganten1998/grimoire-scribe/kpm/preview/manifest.json`
- Stable: `https://raw.githubusercontent.com/ganten1998/grimoire-scribe/kpm/stable/manifest.json`

**These URLs are planned until that first publication.** Add only the channel you want using
KPM's repository settings. A stable release updates stable only; beta/RC updates preview only.
Publication refuses to overwrite release assets with different bytes, refuses to replace a version
with different metadata, and ignores older versions. Rerunning only the publish job reuses the
checked artifact; rebuilding the same published version with different bytes requires a new version.
The channel is written only after release assets are uploaded successfully.

KPM v2 only compares three integers. Grimoire encodes `major.minor.patch` as
`[major, minor, patch*10000+stage]`: beta.N=1000+N, rc.N=5000+N, final=9999 (N=1..999).
So 2.0.0-beta.N appears as KPM 2.0.(1000+N), 2.0.0-rc.N as 2.0.(5000+N) and 2.0.0 as 2.0.9999:
2.0.0-beta.1 is KPM 2.0.1001, and Grimoire still displays 2.0.0-beta.1.
This keeps beta < RC < final < next patch without relying on unsupported version suffixes.

## Contract and validation

The package follows the [official KPM hook contract](https://kindlemodding.org/kindle-dev/kpm/creating-a-package.html)
and [KPM source](https://github.com/KindleModding/KPM): root manifest, POSIX shell hooks, kindlehf
filter, no root filesystem writes. Tests cover deterministic output, platform checks, launcher
ownership, staged failure, install/upgrade/remove, argument forwarding, notebook preservation and
channel ordering. Device evidence is recorded in [TESTING.md](TESTING.md).
