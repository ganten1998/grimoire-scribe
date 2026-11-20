**English** | [日本語](Home-ja) | [Polski](Home-pl)

# Grimoire

**A handwriting notebook for the jailbroken Kindle Scribe.** Lined, dotted, graph and Cornell
paper, sheet music and guitar or bass tab you can mix and stack yourself, nine pens tuned to the
Scribe's pressure and tilt, folders, and three easy ways to get your pages onto a computer as PDF
or PNG. Everything stays on the Kindle in open files: no account, no cloud, no tracking.

It comes dressed like a castle in an old console game: pixel blackletter titles, pixel type,
carved-stone frames and pointed arches (a plain **Classic** style is one tap away in Settings).

Free software under the GPL-3.0. Not affiliated with or endorsed by Amazon.

## Will it run on my Kindle?

You need a **Kindle Scribe** with a jailbreak that runs scriptlets (`.sh` files tapped from the
Library). Current jailbreaks for the Scribe (Vera, SpiderCat, WinterBreak, Sanctuary) all do;
see [kindlemodding.org](https://kindlemodding.org) to jailbreak. On an older jailbreak that runs
no scriptlets, KOReader can open Grimoire instead (see the table).

| Kindle | Status |
|---|---|
| Kindle Scribe (2024, 2nd gen), firmware 5.17 to 5.20.1 | **Works.** Used daily on 5.20.1.0.2 and runs on 5.20.1 (Vera) |
| Kindle Scribe (2022, 1st gen), firmware 5.16.3 or newer | **Works.** Used daily on 5.20.1.0.2 and also runs on 5.20.1 (Vera). If the screen is upside down, see Troubleshooting. |
| Kindle Scribe (2022) on firmware older than 5.16.3 | Should run, but there is no Library launcher for those older jailbreaks yet. With KOReader, it should open from KOReader's Tools > Grimoire, the plugin copied in by hand (see Does it work in KOReader?). |
| Kindle Scribe 3 and Scribe Colorsoft (2025) | **Untested.** No public jailbreak yet. Grimoire already knows their bigger screen (1986x2648): notebooks keep their page size between these and the older Scribes (see Moving between versions). On the Colorsoft's color screen, Grimoire waits after each quick update until the screen's driver has read it, except for the pen's own ink (writing never waits), so buttons should not show striped. |
| Paperwhite, Oasis, Basic and other Kindles without a pen | Not useful: you cannot write without the pen. |

Firmware 5.20.1 is the newest as of October 2026. Check that your jailbreak survives it before
updating; most people keep OTA updates blocked.

## Documentation

| Page | What it covers |
|---|---|
| [Install](Install) | Requirements, the release zip, KPM, updating and uninstalling |
| [Using Grimoire](Using-Grimoire) | The Library, paper, pens, the toolbar, the Index, links, Live view, Wi-Fi, sleep and the passcode |
| [Widgets](Widgets) | Installing, using, updating and removing widgets, and making your own |
| [Fonts and Language](Fonts-and-Language) | English, Spanish, Japanese and Polish, the Classic fonts, and your own fonts |
| [Importing](Importing) | Templates, pictures, PDFs and pages you already have |
| [Getting Pages Off the Kindle](Exporting) | Wi-Fi Share, Effects, USB PDFs and SSH |
| [KOReader](KOReader) | Opening Grimoire from KOReader, annotating books, and the optional plugin |
| [Shortcuts](Shortcuts) | Keyboards and controllers, and `shortcuts.ini` |
| [Troubleshooting](Troubleshooting) | Known issues, common fixes, and moving between versions |
| [Privacy](Privacy) | What stays on the Kindle, and what Share exposes |
| [Contributing](Contributing) | Bugs, feature requests, pull requests, LLM use and the wiki |
| [Building from Source](Building-from-Source) | Toolchains, make targets and tests |
| [Translating](Translating) | Improving the Spanish, Japanese and Polish translations, and adding a language |

## Support Grimoire

Grimoire is free, open source and made by one person, with a lot of care. If it has earned a place
in your notebook, you can support its development on Ko-fi:

<a href="https://ko-fi.com/B0B7EF4TJ"><img src="https://ko-fi.com/img/githubbutton_sm.svg" alt="Support Grimoire on Ko-fi" height="36"></a>

Feedback and feature requests are welcome in the issues too.

## License

Grimoire is free software: GPL-3.0-or-later (see `LICENSE`), the same license as
[FBInk](https://github.com/NiLuJe/FBInk) by NiLuJe, which it uses to drive the e-ink screen.
Much of what Grimoire knows about the Kindle comes from [KOReader](https://koreader.rocks) and
[kindlemodding.org](https://kindlemodding.org).

Kindle and Kindle Scribe are trademarks of Amazon.com, Inc. or its affiliates. Grimoire is an
independent project and is not affiliated with, endorsed by or supported by Amazon.
