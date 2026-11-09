# settings: status_mode=1
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/pdf/folder import
# pdfs: 0
# A folder of PDFs in import/ (import/Work/Report.pdf) puts each one's notebook into the Library folder of that
# name: the Library shows the folder, not a notebook, at its top. (No toast: the Library is in front, but the
# new card is inside the folder, so it says what happened.)
# rust-only
waitbg
wait 300
shot 01-folder
expect toast is Imported notebook (2 pages).
expect library.items is 1
quit
