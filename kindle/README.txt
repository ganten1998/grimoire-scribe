Grimoire @VERSION@: a handwriting notebook for the jailbroken Kindle Scribe
==========================================================================

INSTALL
1. Connect the Kindle to your computer (on a Mac use OpenMTP: https://openmtp.ganeshrvel.com).
2. Copy BOTH files from the "documents" folder here into the Kindle's "documents" folder.
   (The "KOReader plugin (optional)" folder is for installing KOReader's Grimoire plugin by hand:
   see KOREADER below.)
3. Eject. Tap "Grimoire" in your Library (it can take a minute to appear).

UPDATE: copy the new Grimoire.sh over the old one. Notebooks and settings are kept.
GOING BACK to 1.0.0-beta or 1.0.0-rc.1: they set aside pages in a newer format and drop data
they do not know. Read "Moving between versions" in the README (source code link below) first.
FROM QUIRE: install as above and tap Grimoire once; your notebooks move over by themselves.
KOREADER: installing Grimoire leaves KOReader untouched. To add Tools > Grimoire, open
Library > Settings > KOReader > Install / update Grimoire plugin, then restart KOReader.
The plugin opens Grimoire and returns to your book. Quit KOReader before a Library launch.
A Library start maintains a plugin you explicitly installed; a deleted plugin stays deleted.
After an update, open Grimoire once from the Library before using Tools > Grimoire.
On firmware 5.20.1, KOReader can draw an incomplete first screen, also after Grimoire. That is
a KOReader issue (koreader issue 16283), not fixed by Grimoire.
You can also copy the optional grimoire.koplugin folder into koreader/plugins manually.
KPM: the separate .kpkg release asset installs through Kindle Package Manager. It uses the
same app as this zip, preserves notebooks/settings, and installs no KOReader plugin or patches.
SHORTCUTS: Library > Settings > Shortcuts enables optional keyboard/controller button mappings.
Edit grimoire/shortcuts.ini, then Reload shortcuts.ini there. Bluetooth pairing is external;
8BitDo hardware compatibility is not yet confirmed. Analog sticks and hats are not mapped.
LANGUAGE: English or Spanish (a draft), in Settings > Language (Ajustes > Idioma in Spanish).
Automatic, the default, follows the Kindle's own language; pick English there to switch back.
PASSCODE: from the Kindle Library, Grimoire leaves the Device Passcode on. When the Kindle
sleeps, Grimoire hands the screen and touch input to the Kindle's own keypad, and takes them back
after the Kindle reports the unlock. This hand-over still needs confirmation on each firmware.
With a passcode set, the keypad comes before custom sleep pictures and stop-framework mode.
KOReader normally turns the passcode off before Grimoire can run; start Grimoire from the Library
if you need it.
PATCHES: Library > Settings > KOReader installs/updates/removes the plugin and toggles three
optional KOReader patches. Restart KOReader after a change. The manual patches folder includes
fonts and instructions. Terminess now separates the battery icon and percentage in the dropdown
menu. Different patches with the same filename are kept.
WALLPAPER: Display > Sleep screen: KOReader wallpaper uses KOReader's custom image/cover, copied
at each Tools > Grimoire handover. Without one, it shows your page. With a passcode set, the
keypad comes first.
PAGES: the numbered grid reads no previews; Go to page... opens a number pad. In the web reader,
the counter and number field at the top jump directly to any page, one page loaded at a time.

LEAVING: Library > Exit, or hold the power button 4 seconds (the way out when stuck, too):
Grimoire saves and closes (opened from KOReader, Exit reads Back to KOReader, and both go back
there). Never use the passcode reset code 111222777 to get out of anything: it erases the
Kindle, notebooks included.

Your notebooks live in the "grimoire" folder on the Kindle. Closed notebooks are also saved as
PDFs in grimoire/PDFs (Grimoire writes them again, so mark up copies, not these). Library > Share
serves everything over Wi-Fi to any browser: scan its QR code with a phone.

Needs a Kindle Scribe with a jailbreak that runs scriptlets (Vera, SpiderCat, WinterBreak...), or
KOReader with the plugin copied in by hand.
Problems? Attach grimoire/grimoire.log to a bug report:
https://github.com/ganten1998/grimoire-scribe/issues

Grimoire is free software under the GNU GPL version 3 or later (LICENSE.txt). The licenses of
the fonts and code it includes are in grimoire/app/licenses on the Kindle. Source code:
https://github.com/ganten1998/grimoire-scribe
Not affiliated with or endorsed by Amazon. Kindle is a trademark of Amazon.com, Inc.
