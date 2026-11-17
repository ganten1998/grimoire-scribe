**English** | [日本語](../japanese/Install-ja.md) | [Polski](../polish/Install-pl.md)

# Install

Grimoire runs on a **jailbroken Kindle Scribe** with a jailbreak that runs scriptlets (`.sh`
files tapped from the Library). Current Scribe jailbreaks (Vera, SpiderCat, WinterBreak,
Sanctuary) all do; see [kindlemodding.org](https://kindlemodding.org) to jailbreak. On an older
jailbreak with no scriptlets, you can open Grimoire from KOReader instead (see [KOReader](KOReader)).

## From the release zip

1. Download `Grimoire-<version>.zip` from the
   [Releases](https://github.com/ganten1998/grimoire-scribe/releases) page and unzip it. You get a
   `documents` folder with two files, plus optional KOReader plugin and patch folders for a manual
   installation.
2. Connect the Kindle to your computer. The Scribe uses MTP: on a Mac, use
   [OpenMTP](https://openmtp.ganeshrvel.com) (or `brew install --cask openmtp`); on Windows and
   Linux it shows up as a device.
3. Copy **both files** into the Kindle's `documents` folder:
   - `Grimoire.sh`, the app (it carries everything inside it)
   - `Grimoire - reset pen calibration.sh`, a rescue switch you will hopefully never need
4. Eject the Kindle. **Grimoire** appears in your Library with a gothic cover (indexing can take
   a minute). Tap it.

The first launch takes a few seconds while it unpacks itself into a `grimoire` folder. Your
notebooks and settings live there, under `/mnt/us/grimoire` on the Kindle.

## Updating

Copy the new `Grimoire.sh` over the old one; your notebooks and settings stay. Going back to an
older version works the same way (see [Troubleshooting](Troubleshooting#moving-between-versions)).

You can also update from the app: **Library > Settings > Updates** checks Grimoire's feed through
KPM and installs a newer Grimoire. Grimoire closes, KPM installs it (its progress on the screen),
and Grimoire starts again. Betas get betas and releases; a tap there switches to releases only. It
needs KPM and Wi-Fi.

## Coming from Quire

Grimoire was called Quire. Copy the new files and tap Grimoire once. It moves your notebooks and
settings from `quire/` to `grimoire/` and removes the old Quire launcher.

## Installing with KPM

The release also includes a KPM package for `kindlehf` Scribes. Copy the `.kpkg` to the Kindle,
close Grimoire, and from a shell on the Kindle:

```sh
/var/local/kmc/bin/kpm -y install file:///mnt/us/Downloads/grimoire_2.0.1001_kindlehf.kpkg
/var/local/kmc/bin/kpm launch grimoire
```

Replace the filename with the asset you downloaded. The Library launchers call
`kpm launch grimoire`; the complete app stays in KPM's package directory and unpacks into
`grimoire/app` on first launch. A reset-calibration Library item is included. Quit KOReader before
a normal Library launch.

KPM installation can migrate the two official manual Grimoire launchers. Only unchanged KPM
Library launchers are removed on uninstall; **all `grimoire/` data remains**, including notebooks,
settings, PDFs and mappings.

## Uninstall

Delete `Grimoire.sh` and `Grimoire - reset pen calibration.sh` from `documents`, then the
`grimoire` folder (copy `grimoire/notebooks` or `grimoire/PDFs` somewhere first if you want to
keep your pages). With KOReader, delete `koreader/plugins/grimoire.koplugin` too.

With KPM: `kpm -y uninstall grimoire`. That removes only unchanged KPM Library launchers and
leaves all `grimoire/` data in place; remove the data folder yourself only when you intend to
delete it.
