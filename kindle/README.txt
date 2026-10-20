Grimoire @VERSION@: a handwriting notebook for the jailbroken Kindle Scribe
==========================================================================

INSTALL
1. Connect the Kindle to your computer (on a Mac use OpenMTP: https://openmtp.ganeshrvel.com).
2. Copy BOTH files from the "documents" folder here into the Kindle's "documents" folder.
3. Eject. Tap "Grimoire" in your Library (it can take a minute to appear).

UPDATE: copy the new Grimoire.sh over the old one. Notebooks and settings are kept.
GOING BACK to 1.0.0-beta or 1.0.0-rc.1: they set aside pages in a newer format and drop data
they do not know. Read "Moving between versions" in the README (source code link below) first.
FROM QUIRE: install as above and tap Grimoire once; your notebooks move over by themselves.
KOREADER: Grimoire is a separate app. Quit KOReader first, then tap Grimoire in the Library.
It will not start from KOReader's file browser, because KOReader keeps the pen and the screen.

LEAVING: Library > Exit, or hold the power button 4 seconds (the way out when stuck, too):
Grimoire saves and closes. Never use the passcode reset code 111222777 to get out of anything:
it erases the Kindle, notebooks included.
PASSCODE: a Device Passcode is paused while Grimoire is open and put back when you leave.

Your notebooks live in the "grimoire" folder on the Kindle. Closed notebooks are also saved as
PDFs in grimoire/PDFs (Grimoire writes them again, so mark up copies, not these). Library > Share
serves everything over Wi-Fi to any browser: scan its QR code with a phone.

Needs a Kindle Scribe with a jailbreak that runs scriptlets (Vera, SpiderCat, WinterBreak...).
Problems? Attach grimoire/grimoire.log to a bug report:
https://github.com/ganten1998/grimoire-scribe/issues

Grimoire is free software under the GNU GPL version 3 or later (LICENSE.txt). The licenses of
the fonts and code it includes are in grimoire/app/licenses on the Kindle. Source code:
https://github.com/ganten1998/grimoire-scribe
Not affiliated with or endorsed by Amazon. Kindle is a trademark of Amazon.com, Inc.
