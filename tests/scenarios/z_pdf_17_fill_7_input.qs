# settings: status_mode=1
# settings: shortcuts_enabled=1
# root: z_pdf_11_fill_1_book
# env: GRIMOIRE_IMPORT_STABLE=fast GRIMOIRE_FILL_STEP_PAGES=1
# copy: tests/fixtures/pdf/tabs import
# pdfs: 1
# rust-only
# The input waits while a fill's pages go into their notebook (2.2). The planner of z_pdf_12 again, its commit
# slowed to a file a tick (GRIMOIRE_FILL_STEP_PAGES): while the book's notice counts, O (Settings, on a keyboard)
# and a tap on the notebook's card do nothing: the Library stays as it is, with no sheet and no toast. Once the
# pages are in, the card opens the notebook, filled.
waitfor notice has Adding book pages:
# (one batch: the key, then the tap)
key 24 0
tap library card.1
expect notice has Adding book pages:
expect screen is library
expect library.sheet is none
expect toast is
waitfor toast is Added 2 book pages.
waitbg
wait 300
tap library card.1
wait 700
expect screen is editor
expect editor.pages is 4
quit
