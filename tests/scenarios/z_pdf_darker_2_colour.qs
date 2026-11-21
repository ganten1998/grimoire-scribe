# settings: status_mode=1
# settings: pdf_gamma=2.0
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/pdf/colour import
# pdfs: 0
# Darker lines at Strong (settings.ini pdf_gamma=2.0, 2.2, issue #173) on the colour PDF of z_pdf_2 dropped in
# import/: its two gray pages are drawn darker than at Off (z_pdf_darker_3_off's levels), and its colour page,
# kept in colour, is exactly what Off makes. The notebook says gamma 2.0.
# rust-only
waitbg
wait 300
expect library.items is 1
tap library card.1
wait 700
expect screen is editor
expect editor.pages is 3
expect annotate.gamma is 2.0
expect annotate.luma is 239
shot 01-gray-page-strong
pentap panel next
wait 500
expect editor.page is 2
expect annotate.luma is 230
shot 02-colour-page-as-it-is
pentap panel next
wait 500
expect editor.page is 3
expect annotate.luma is 230
pentap panel library
wait 600
quit
