**English** | [日本語](../japanese/Importing-ja.md)

# Importing Templates, Pictures and PDFs

## Templates

Copy PNG or JPG pictures into `grimoire/templates/` over USB while Grimoire is closed (upright,
3:4, 1860 x 2480 is best; light lines on white work best on e-ink). In a notebook, **Paper > More
layouts...** shows them; **New notebook** has the same button.

A template is ready to pick once its tile shows its picture (Grimoire checks each new file in the
background first). Each notebook keeps its own copy of a template it uses.

## Photos as paper

Copy JPG or PNG photos into `grimoire/photos` over USB. In a notebook, open **Paper** and pick
**Photos...**: the photo becomes the page's paper, fitted to the page and turned upright the way
your phone took it, and you write on it like any page (Undo puts the old paper back). For a page of
its own, add a new page in Pages first.

## Pages you already have

Put a folder of page pictures (PNG or JPG, one per page, such as a PDF exported as pictures with
`pdftoppm -gray -png -r 200 file.pdf page`) in `grimoire/import/<Name>/` and start Grimoire: it
becomes the notebook `<Name>`, and the folder moves to `grimoire/import/done/`.

## PDFs

Copy a PDF (a planner, a book) into `grimoire/import/` over USB, or over SSH while Grimoire runs,
and open the Library: a notice at the bottom says how far it is ("Importing: page 37 of 412"), and
the notebook appears. Its pages are the PDF's pages, in colour where the PDF has colour (the
Scribe shows them in gray). A PDF put in a folder there goes into the Library folder of that name,
and the PDF moves to `import/done/`.

Up to 2000 pages and 512 MB; a longer or bigger PDF, or one with a password, is refused with a
message, and the same PDF a second time says "PDF already imported.". A page that cannot be read
becomes a blank page and the end says so ("Some pages couldn't be read."); a PDF with no page that
reads is not imported. A PDF that fails waits until you replace it (three tries); one that found
the storage full goes once there is room.

KOReader's **Open this book** hands its PDF to this same import (see [KOReader](KOReader)), and
still draws DjVu and comic books itself.

## A file anywhere on the Kindle

**Library > New > From a file...** opens Files, titled "Import a PDF or pictures":

- **Import** (`grimoire/import`, with what each file is doing: Waiting, "Importing: page 40 of
  365", Imported, and a Look again row)
- **Documents**
- **Downloads**
- **All storage** (Grimoire's notebooks, PDFs and trash are never listed)

A tap on a folder opens it; a tap on a PDF opens its card ("14.2 MB, Oct 30" and Import as
notebook, or Open its notebook when a notebook already holds that book; "has notes" shows on its
row), and the second tap imports it.

A notebook made from a PDF remembers its file, so the card of such a PDF offers Import as notebook
beside Open its notebook: the very same file says "PDF already imported.", and another file that
only looks like the book to KOReader's key (an edited copy, say) becomes a notebook of its own. A
folder of pictures is imported from its `...` (or a finger held on it) card. The notebook takes
the file's name unless you typed one in the New notebook sheet first, and goes in that sheet's
folder.

The file stays exactly where it was: Grimoire only reads outside its own folders, and never
follows a link (a file picked in Import itself is filed in `import/done/` like one dropped
there). A file picked while an import from an earlier run is still going is imported once that
one is done.

## Erased ink leaves the files

When a notebook closes, ink erased entirely is taken out of its page files in the background. A
stroke erased only in part stays whole in the file.
