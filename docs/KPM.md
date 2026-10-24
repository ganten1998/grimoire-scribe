# KPM installation and release pipeline

Grimoire packages the tested self-contained ZIP launcher as a KPM v2 `.kpkg`. The initial target
is the **Kindle Scribe on kindlehf**. KPM's platform field cannot restrict the package to Scribe
models; other kindlehf Kindles do not gain pen support by installing it.

Download the `.kpkg` from the release and copy it to the Kindle. From an existing Kindle shell:

```sh
/var/local/kmc/bin/kpm -y install file:///mnt/us/Downloads/grimoire_1.1.5001_kindlehf.kpkg
/var/local/kmc/bin/kpm launch grimoire
```

Replace the filename with the asset actually downloaded. The Library launchers call `kpm launch
grimoire`; the complete app stays in KPM's package directory and unpacks into `grimoire/app` on
first launch. A reset-calibration Library item is included. Close Grimoire before installing,
upgrading or removing it. Quit KOReader before a normal Library launch.

Installation can migrate the two official manual Grimoire launchers. It stages both before
replacing either, restores them if a hook fails, and refuses unrelated documents with those
names. An existing running Grimoire blocks the hook. KPM itself controls package extraction and
upgrade rollback; the hook cannot make KPM's own package replacement transactional.

Uninstall with `kpm -y uninstall grimoire`. Only unchanged KPM Library launchers are removed.
**All `grimoire/` data remains**, including notebooks, settings, PDFs, mappings and unpack cache.
That cache may still serve an explicitly installed KOReader plugin or manual installation.
Remove the data folder yourself only when you intend to delete it. KPM installation/removal never
installs or removes KOReader plugins, patches, fonts, PIN flags or root filesystem files.

## Updating

Once the preview/stable feed is live, add the chosen manifest URL with `kpm add-repo URL` and use
`kpm -y install grimoire` to install or update Grimoire specifically. Repository-based beta → RC
upgrades were tested on the actual KPM 0.2.2 device.

**KPM 0.2.2 local-file limitation:** installing a second `file://...kpkg` over an already installed
package can fail in KPM's dependency resolver before Grimoire's hooks run. Its `upgrade` command
also skips locally installed packages. For a manual-file update, download the new package first,
close Grimoire, then use:

```sh
/var/local/kmc/bin/kpm -y uninstall grimoire
/var/local/kmc/bin/kpm -y install file:///mnt/us/Downloads/grimoire_1.1.5001_kindlehf.kpkg
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

The first published release through this workflow creates these feeds:

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
Thus 1.1.0-rc.1 appears as KPM 1.1.5001; Grimoire still displays 1.1.0-rc.1.
This keeps beta < RC < final < next patch without relying on unsupported version suffixes.

## Contract and validation

The package follows the [official KPM hook contract](https://kindlemodding.org/kindle-dev/kpm/creating-a-package.html)
and [KPM source](https://github.com/KindleModding/KPM): root manifest, POSIX shell hooks, kindlehf
filter, no root filesystem writes. Tests cover deterministic output, platform checks, launcher
ownership, staged failure, install/upgrade/remove, argument forwarding, notebook preservation and
channel ordering. Device evidence for this candidate is recorded in TESTING-rc1.md.
