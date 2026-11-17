**English** | [日本語](../japanese/KOReader-ja.md) | [Polski](../polish/KOReader-pl.md)

# KOReader

Yes: KOReader can open Grimoire and get you back to your book.

## Installing the plugin

With KOReader installed (the Kindle's `koreader`, `extensions/koreader` or `documents/koreader`
folder), open **Grimoire > Settings > KOReader > Install / update Grimoire plugin**, then restart
KOReader. That adds **Tools > Grimoire** to its menu. A fresh Grimoire installation leaves
KOReader untouched.

Open Grimoire is also an action for KOReader's gestures (**Settings > Taps and gestures**). The
menu speaks KOReader's language, English, Spanish, Japanese or Polish (**Abrir Grimoire**, **PDF de
los cuadernos**, **Restablecer la calibración del lápiz**; **Grimoireを開く**, **ノートのPDF**,
**ペンのキャリブレーションをリセット**; **Otwórz Grimoire**, **PDF-y notatników**, **Przywróć
kalibrację rysika**). Only a start from the Library updates the plugin, so
after you update Grimoire, tap it in the Library once.

This needs a KOReader started from its Library item or from KUAL (KOReader's `koreader.sh`
start). A KOReader launcher that leaves the Kindle interface running is supported too: Grimoire
pauses it for its own session and restores it before returning.

## Opening and leaving

- **Open Grimoire**: KOReader closes and Grimoire opens in its place, without the Kindle's home
  screen in between. In Grimoire, **Library > Exit** reads **Back to KOReader**; it, or holding
  the power button 4 seconds, brings KOReader back with the book you had open (or its file
  browser).
- **Notebook PDFs**: KOReader's file browser in `grimoire/PDFs`.
- **Reset pen calibration**: the same rescue as the Library's reset item.

Opened from KOReader, Grimoire cannot show the Kindle's own screensaver as its sleep screen (that
needs the Kindle's screens, which KOReader keeps paused), so it shows your page instead.

**Display > Sleep screen: KOReader wallpaper** uses a snapshot of KOReader's configured custom
image, random image or book cover, exported whenever **Tools > Grimoire** hands over. It also
works on a later Library start. If no custom image is available, your page shows. A random folder
picks one image per handover; KOReader's message/status widgets are not copied. With a Device
Passcode set, the passcode keypad comes first.

*Execute shell script* on `Grimoire.sh` in KOReader's file browser still refuses, because KOReader
keeps the pen and the touch screen to itself (KOReader reports status 3): use **Tools > Grimoire**
instead.

## Annotate this page

In a book in KOReader, **Tools > Grimoire > Annotate this page in Grimoire** (also a gesture
action) opens the page you are reading in Grimoire, ready to write on. Grimoire keeps one notebook
per book, named after its title (leaving out characters it cannot show), with the pages you
annotate in the book's order; annotating the same page again opens it with what you wrote.
**Library > Back to KOReader** returns to the same book and page.

A PDF's page is the whole page; an EPUB's is the screen as KOReader lays it out, so after a change
of font size a page may come in again as a new one. A notebook's own PDF from `grimoire/PDFs` is
refused, since annotating it would make a notebook of a notebook: open that notebook in Grimoire
instead.

## Open this book in Grimoire

In a PDF, DjVu or comic book, **Tools > Grimoire > Open this book in Grimoire** (also a gesture
action) puts every page of the book into its Grimoire notebook at once, then opens the page you
are reading. In KOReader's file browser, long-press such a file and choose **Open with... >
Grimoire** to do the same without opening it.

For a DjVu or comic book KOReader draws each page first, about a second or two a page on a Scribe,
with a message that a tap stops. A PDF goes straight to Grimoire instead, which reads it itself as
it imports a PDF (in colour where the PDF has colour; the notice at the bottom counts its pages,
and the Library stays usable meanwhile) and then opens the page you were reading.

If you opened something else in the meantime, nothing opens by itself: a toast says the import is
done, the book's notebook is a card in the Library, and it opens at that page when you tap it.
Pages the notebook already has, with what you wrote on them, are kept as they are, so opening the
same book again only adds what is missing. It is the same notebook **Annotate this page** uses. A
PDF's own links come along (a planner's tabs). A PDF that needs a password still goes the older
way, KOReader drawing it, and so does a PDF where Grimoire does not read (a hidden folder,
KOReader's own folders) or one over 512 MB.

## Document links

The tabs of a planner PDF are part of its pages. A tap follows one (a link you drew yourself comes
first where they overlap); a finger held on one starts Select, as anywhere on the page. Clear
page keeps them, a lasso around what you wrote in a day cell never takes the cell's link, and
Copy page, Move to... and Copy to notebook... carry them (to pages that go too). The Links bar
lists them as one row, "From the PDF (N)", which opens them grouped by the page they go to; Areas
draws their boxes dotted. They are in a file of their own (`doclinks.ini`), so an older Grimoire
shows the pages and your links without them.

## Photos

Copy JPG or PNG photos into `grimoire/photos` over USB. In a notebook, open Paper and pick
**Photos...**: the photo becomes the page's paper, fitted to the page and turned upright the way
your phone took it, and you write on it like any page (Undo puts the old paper back). For a page
of its own, add a new page in Pages first.

## The optional patches

**Library > Settings > KOReader** installs or updates the plugin, removes it, or toggles each of
the three optional patches: diagonal screenshot gestures off, square folder corners, and
Terminess menu fonts. The font patch includes the font. Restart KOReader after changing a patch or
the plugin. A different patch at the same filename is preserved; no patches are applied just by
updating Grimoire.

The plugin is optional. To remove it, delete `koreader/plugins/grimoire.koplugin`; while KOReader
stays installed, Grimoire does not put it back. To put it back, or into a KOReader installed
again since, or where the Library cannot run scriptlets, copy the zip's `KOReader plugin
(optional)/grimoire.koplugin` folder into the Kindle's `koreader/plugins` (with `Grimoire.sh` in
`documents` as usual). Never copy a folder named `koreader` onto the Kindle: it could replace your
KOReader.
