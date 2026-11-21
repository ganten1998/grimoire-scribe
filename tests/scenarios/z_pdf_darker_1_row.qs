# settings: status_mode=1
# copy: tests/fixtures/pdf/pick Documents
# pdfs: 0
# Darker lines in imported PDFs (2.2, issue #173): Settings > System's row says Off and opens its list of three; Back
# changes nothing; Medium is picked, saved at once, and the row says it. A PDF picked from then on (the runner's
# pdfimport command: Files' entry) draws its gray pages at gamma 1.5: its first page, the planner's page 1, is darker
# than at Off (z_pdf_darker_3_off's page 1), and its notebook says 1.5. Off again: no line in settings.ini.
# rust-only
tap library settings
wait 100
expect library.sheet is settings
tap library settings.cat.6
wait 100
expect settings.pdf_gamma is 1.0
shot 01-system
tap library darker-lines
wait 100
expect library.sheet is darker-lines
shot 02-list
tap library darker-lines.back
wait 100
expect library.sheet is settings
expect settings.pdf_gamma is 1.0
tap library darker-lines
wait 100
tap library darker-lines.1
wait 100
expect library.sheet is settings
expect settings.pdf_gamma is 1.5
shot 03-medium
tap library done
wait 100
expect library.sheet is none
pdfimport z_pdf_darker_1_row/Documents/Planner.pdf
waitbg
wait 300
expect library.items is 1
tap library card.1
wait 700
expect screen is editor
expect annotate.gamma is 1.5
expect annotate.luma is 241
shot 04-page-1-medium
pentap panel library
wait 600
expect screen is library
tap library settings
wait 100
tap library settings.cat.6
wait 100
tap library darker-lines
wait 100
tap library darker-lines.0
wait 100
expect settings.pdf_gamma is 1.0
tap library done
wait 100
quit
