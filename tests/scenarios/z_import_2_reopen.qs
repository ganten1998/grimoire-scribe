# Run after z_import_1_startup on the same root: the folder it imported went to import/done, so
# this start imports nothing (no toast but the first-run tip), and the notebook opens where it was
# left, on the blank page it added.
# root: z_import_1_startup
wait 1500
expect toast not Imported a notebook of 3 pages.
expect screen is library
expect library.items is 1
tap 326 608
wait 600
shot 01-reopened
expect screen is editor
expect editor.pages is 4
expect editor.page is 4
expect editor.paper is 4
quit
