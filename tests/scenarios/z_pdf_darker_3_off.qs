# settings: status_mode=1
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/pdf/colour import
# pdfs: 0
# Darker lines in imported PDFs (2.2, issue #173) left at Off, the default: the colour PDF of z_pdf_2 (gray, colour,
# gray) is imported as before, and each page's mean luma is pinned here as the level the other darker lines
# scenarios compare with. Its page 1 is the planner's page 1 (z_pdf_darker_1_row picks a planner whose page 1 is
# drawn the same); its notebook says no gamma.
# rust-only
waitbg
wait 300
expect library.items is 1
tap library card.1
wait 700
expect screen is editor
expect editor.pages is 3
expect annotate.gamma is 1.0
expect annotate.luma is 244
pentap panel next
wait 500
expect editor.page is 2
expect annotate.luma is 230
pentap panel next
wait 500
expect editor.page is 3
expect annotate.luma is 238
pentap panel library
wait 600
quit
