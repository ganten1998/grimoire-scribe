# Grimoire Enhanced and an Emacs / org-roam bridge

Source investigation, 2026-10-24. This is a follow-up design, not features claimed by beta.2.
The beta.2 work implements the launcher, PIN, navigation, optional patches and wallpaper changes
in CHANGELOG.md. KOReader was inspected through its open source and Grimoire's executable plugin
harness; a full KOReader session on additional Kindle firmware was not available for this run.

## What KOReader already provides

Sources inspected at KOReader commit
[`afff28b107a7ea4ab5867b5e65dd054987ca1d99`](https://github.com/koreader/koreader/tree/afff28b107a7ea4ab5867b5e65dd054987ca1d99):

- [ReaderHighlight](https://github.com/koreader/koreader/blob/afff28b107a7ea4ab5867b5e65dd054987ca1d99/frontend/apps/reader/modules/readerhighlight.lua)
  exposes `addToHighlightDialog(key, callback)` and `removeFromHighlightDialog(key)`. The
  selection menu calls the callback with its highlight module and an optional annotation index.
  This is a real extension point for an **Annotate selection in Grimoire** button. The saved
  highlight editor has a separate implementation; this hook must not be assumed to cover both.
- [ReaderAnnotation](https://github.com/koreader/koreader/blob/afff28b107a7ea4ab5867b5e65dd054987ca1d99/frontend/apps/reader/modules/readerannotation.lua)
  keeps structured annotations with positions, selected text, notes and chapter information.
  EPUB annotations use text XPointers and can recompute page numbers after layout changes. Its
  annotation migration/import code is a strong reason to use its APIs rather than edit sidecars
  under a running KOReader.
- [Exporter](https://github.com/koreader/koreader/blob/afff28b107a7ea4ab5867b5e65dd054987ca1d99/plugins/exporter.koplugin/main.lua)
  already supports JSON and Markdown targets. An Emacs importer can start with those instead of
  inventing another extraction format or scraping screenshots for text.
- [ReaderLink](https://github.com/koreader/koreader/blob/afff28b107a7ea4ab5867b5e65dd054987ca1d99/frontend/apps/reader/modules/readerlink.lua)
  has external-link button extension points. That offers a later route for **Open in org-roam**
  links, after the note viewer exists; automatic custom-scheme navigation needs its own tests.
- [userpatch](https://github.com/koreader/koreader/blob/afff28b107a7ea4ab5867b5e65dd054987ca1d99/frontend/userpatch.lua)
  can register a patch against a plugin each time it is loaded. Grimoire's own plugin is a better
  home for long-lived features than a patch that rewrites KOReader core files.

## A useful first Enhanced feature

Select a passage, choose **Annotate selection in Grimoire**, write, then return to that passage.
Keep the original quote, document identity and anchor next to the ink. Export the quote and a
handwriting PNG/PDF attachment together into an org-roam note.

Implementation boundaries:

1. Register a selection-menu button from the Grimoire plugin after ReaderUI's modules initialize.
   Capture the quote and positions before closing the menu/book; those objects do not survive
   the existing restart handover. Use the current plugin's teardown/launch path for input safety.
2. Use the book's partial MD5 plus a PDF page/region or EPUB start/end XPointer as source identity.
   A visible EPUB page number alone is unstable when fonts, margins or orientation change.
3. Preserve a render fingerprint with each ink snapshot. A stable text anchor finds the passage,
   but cannot magically relocate handwriting onto a newly reflowed page. Keep the old snapshot
   and its ink; create another snapshot when the layout differs. Never silently repaint its paper.
4. Store bounded, versioned selection metadata alongside `source.ini`/attachments. Add a plugin
   capability for the extended request. Keep the .qpg header/tool registry unchanged, and preserve
   metadata that an older app does not understand. The current page-number mapping is not enough
   to implement this safely on its own.
5. Add a return request that the restarted plugin consumes only after ReaderUI is ready. Resolve
   the book and text anchor there. Keep private book paths and selected text out of diagnostic logs.

A subsequent feature can add a passage's linked handwritten note to KOReader's annotation list.
Avoid promising editable native PDF ink initially: Grimoire's stroke model, a flattened export,
and a PDF ink annotation are different representations. Export a separate annotated copy first.

## Wireless org-roam viewer and basic editor

The [org-roam manual](https://www.orgroam.com/manual.html) makes the right ownership model clear:
Org files are the durable notes; its database is an auxiliary index. Emacs should own edits and
indexing. The Kindle can be a small client without running an entire Emacs or SQLite writer.

A staged pipeline:

- **Browse:** Emacs exports a compact index of node IDs, titles, aliases, tags, links and backlinks.
  A Kindle client downloads it over Wi-Fi and caches note bodies on demand. Use org-roam IDs,
  not filenames, as identity. Search and backlink lists suit e-ink better than an animated graph.
- **Capture first:** create a note or append a short thought, quote or handwriting attachment.
  Emacs inserts it through Org buffers and refreshes its index. This gives useful writing with
  fewer conflicts than replacing a whole note from two computers.
- **Basic editing:** edit a note's text, TODO state or checkboxes. Each request carries its base
  revision and a unique operation ID. Emacs rejects conflicting revisions and keeps both texts;
  retries cannot duplicate captures. Preserve property drawers, IDs, links and heading boundaries.
  Start with file-level nodes before implementing edits inside arbitrary heading nodes.
- **Offline:** retain a local outbox until Emacs acknowledges each operation. A reconnect must
  never discard unsent text or overwrite a newer desktop edit. Show conflicts as an explicit task.
- **Handwriting:** upload an ink snapshot plus its quote/anchor manifest; Emacs saves it as an Org
  attachment and inserts the link. OCR can be an optional desktop step, not a prerequisite or
  something that replaces the ink. An org-roam note can link back to its KOReader passage.

For the first client, a KOReader plugin can reuse its text widgets and menus. A later native
Grimoire note browser can use the same protocol. The current Grimoire naming keyboard has a
96-byte single-line limit; it is not a sufficient editor for Org notes. A real multiline editor
and cursor/selection handling are needed before claiming basic editing.

A small authenticated companion service should expose IDs and operations, not arbitrary filesystem
paths or an unrestricted Emacs evaluation endpoint. Pair a Kindle once with a generated token;
keep the service on the local network or a private tunnel and avoid copying encrypted notes into
an unencrypted cache by default. Emacs remains the single writer of the org-roam index.

## Before shipping Enhanced

Test the selection hook against real KOReader modules and a Kindle, not only mocks. Cover PDF and
EPUB anchors, changed fonts/layouts, repeated annotations, missing books, restart/return position,
failed writes, process exits, ownership of input and private logging. For the Emacs bridge, add
round-trip tests for UTF-8 Org text, IDs/property drawers, revision conflicts, reconnect/retry
idempotency and attachments. Keep these features separate from the beta.2 release claim.
