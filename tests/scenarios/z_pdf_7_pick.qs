# settings: status_mode=1
# copy: tests/fixtures/pdf/pick Documents
# pdfs: 0
# A PDF picked anywhere (the picker's entry, here the runner's pdfimport command: lane L8 draws the picker) is
# read in place and never moved, takes the name given and goes into the folder given. The Library shows the folder.
# rust-only
pdfimport z_pdf_7_pick/Documents/Planner.pdf My_planner Reading
waitbg
wait 300
shot 01-picked
expect library.items is 1
quit
