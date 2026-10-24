# 1.1.0-beta.2 device checks

The host checks cover the logic, package, launcher ordering and failures. These steps require a
real Kindle; this release does not claim that the new native PIN path has passed them on every
firmware. Keep a notebook backup and use Library > Exit or hold power for 4 seconds to leave.

1. **KOReader handover.** Install the new Grimoire.sh and launch it from the Kindle Library once,
   or use Settings > KOReader > Install / update Grimoire plugin. Restart KOReader. From an open
   PDF and an EPUB, try Tools > Grimoire > Open Grimoire, then Annotate this page. Write, return,
   and confirm the same book and position. Annotate the same page again and keep the existing ink.
   Test both an ordinary launch and KUAL's no-framework launch if available. Nonstandard installs
   supported automatically: extensions/koreader and documents/koreader.
2. **Native PIN.** Enable Device Passcode in Kindle Settings and launch Grimoire from the Library,
   with KOReader closed. Write in a notebook, sleep and wake. The real Kindle keypad should appear;
   waiting or touching it must never open the notebook. After the correct PIN, the notebook and ink
   should return. Try portrait/landscape, a second sleep, and sleeping while the pen is still down.
   The next pen stroke and first finger tap after unlock should work without stray ink. Exit from
   the lock screen with a four-second hold and check the native UI/PIN still works. Native PIN
   overrides a custom sleep picture and stop-framework mode. A KOReader-hosted session normally
   inherits KOReader's disabled-PIN policy; it is not the native PIN test.
3. **Many pages.** Open a large notebook with heavily inked pages. Open Pages, move between grid
   screens, select/unselect pages, cancel and use Go to page... for an early and a late page.
   The number grid should appear without rendering thumbnails. Delete disposable test pages only.
4. **Browser.** Share a large notebook to a phone and computer. The top field should show the
   current page and total, accept a direct jump or Enter, and load only that page. Check Previous,
   Next, browser Back, a full-size PNG, PDF, zip and Effects. Try a narrow phone screen.
5. **Patches.** In Library > Settings > KOReader, toggle each patch and restart KOReader. Check
   screenshot gestures, mosaic folder corners and menu fonts. Disable them and restart: defaults
   should return. Ordinary book fonts should remain unchanged. Remove the Grimoire plugin from
   this sheet and verify it stays removed at another Library start; reinstall it from the sheet.
6. **Wallpaper.** Set a custom picture, random image folder or book cover in KOReader. Open
   Grimoire through its plugin, then select Display > Sleep screen: KOReader wallpaper. Sleep in
   portrait and landscape. Confirm the chosen image, and that the page returns on wake. Disable
   the custom image in KOReader and hand over again: Grimoire should fall back to the current page.
   The image is a snapshot per handover, not KOReader's live message/status screen.

For a failure, record the Scribe model, firmware, jailbreak, KOReader version, installation folder,
and launch method. Attach grimoire/grimoire.log and, for a handover failure, koreader/crash.log.
Review logs for private material before sharing; do not include a PIN, serial number or book file.
