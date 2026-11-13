# settings: status_mode=1
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/scen-import import
# copy: tests/fixtures/pdf/mixed import/Etudes Op. 10
# pdfs: 0
# A folder that holds PDFs and page pictures imports its PDFs first, into the Library folder of its name, and
# then, with the PDFs done and moved, its pictures as one notebook named after the folder, as before: two
# cards in the Library, the folder and the notebook.
# rust-only
waitfor library.items is 2
shot 01-both
waitbg
wait 300
expect library.items is 2
quit
