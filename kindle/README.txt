Grimoire @VERSION@: a handwriting notebook for the jailbroken Kindle Scribe
==========================================================================

INSTALL
1. Connect the Kindle to your computer (on a Mac use OpenMTP: https://openmtp.ganeshrvel.com).
2. Copy BOTH files from the "documents" folder here into the Kindle's "documents" folder.
3. Eject. Tap "Grimoire" in your Library (it can take a minute to appear).

UPDATE: copy the new Grimoire.sh over the old one. Notebooks and settings are kept.
FROM QUIRE: install as above and tap Grimoire once; your notebooks move over by themselves.
KOREADER: Grimoire is a separate app. Quit KOReader first, then tap Grimoire in the Library.
It will not start from KOReader's file browser, because KOReader keeps the pen and the screen.
PASSCODE: if your Kindle has a Device Passcode, Grimoire pauses it while Grimoire is open (as
KOReader does), so the Kindle does not lock, and turns it back on when you leave. If the Kindle
restarts while Grimoire is open, the passcode stays off until you turn it back on in the Kindle's
Settings; Grimoire reminds you when it next starts.

Your notebooks live in the "grimoire" folder on the Kindle. Closed notebooks are also saved as
PDFs in grimoire/PDFs. Library > Share serves everything over Wi-Fi to any browser.

Needs a Kindle Scribe with a jailbreak that runs scriptlets (Vera, SpiderCat, WinterBreak...).
Problems? Attach grimoire/grimoire.log to a bug report:
https://github.com/ganten1998/grimoire-scribe/issues

Grimoire is free software under the GNU GPL version 3 or later (LICENSE.txt). The licenses of
the fonts and code it includes are in grimoire/app/licenses on the Kindle. Source code:
https://github.com/ganten1998/grimoire-scribe
Not affiliated with or endorsed by Amazon. Kindle is a trademark of Amazon.com, Inc.
