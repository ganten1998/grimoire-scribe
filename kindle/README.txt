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
KOREADER: Grimoire is a separate app; tap it in the Library. If KOReader is installed, tapping
Grimoire once also adds Tools > Grimoire to KOReader, which opens Grimoire and comes back to your
book. (Without the plugin, quit KOReader first: Grimoire.sh does not run from KOReader's file
browser.) After an update, tap Grimoire in the Library once: that updates the plugin too. For
setups without Library scriptlets, or to install the plugin again, copy the folder "KOReader
plugin (optional)" from this zip: its grimoire.koplugin folder goes into koreader/plugins on the
Kindle. You can also manage it in Library > Settings > KOReader. To remove it by hand, delete
koreader/plugins/grimoire.koplugin; while KOReader stays
installed, Grimoire does not put it back (copying the folder again does).
LANGUAGE: English or Spanish (a draft), in Settings > Language (Ajustes > Idioma in Spanish).
Automatic, the default, follows the Kindle's own language; pick English there to switch back.
PASSCODE: from the Kindle Library, Grimoire keeps the native PIN enabled. Sleep hands input
and screen to the Kindle's keypad; unlock to return to your notebook. Native PIN takes priority
over custom sleep pictures and stop-framework mode. KOReader normally disables the PIN before
Grimoire can run; start Grimoire from the Library if you need native PIN protection.
PATCHES: Library > Settings > KOReader installs/updates/removes the plugin and toggles the three
optional KOReader patches. Restart KOReader after a change. The manual patches folder includes
fonts and instructions. Different patches with the same filename are kept.
WALLPAPER: Display > Sleep screen: KOReader wallpaper uses KOReader's custom image/cover, copied
at each Tools > Grimoire handover. Without one, it shows your page. Native PIN takes priority.
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
