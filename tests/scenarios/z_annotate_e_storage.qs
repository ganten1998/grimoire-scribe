# settings: status_mode=1
# env: GRIMOIRE_HOST=koreader
# env: GRIMOIRE_ANNOTATE=1
# copy: tests/fixtures/annotate/storage .
# Page 5 of a book whose notebook is there (notebook.ini's source=), but nothing can be written into
# it (its bg is a file, not a folder, as a full or damaged storage refuses a write): the page is not
# added, the notebook stays as it was, the request goes, and the toast says why. Grimoire starts on
# its Library, the notebook still one page.
wait 300
shot 01-toast
expect screen is library
expect annotate.result is failed
expect annotate.leftover is 0
expect library.items is 1
expect toast is Could not add the page KOReader sent. Is the storage full?
tap 326 608
wait 600
expect screen is editor
expect editor.pages is 1
expect editor.paper is 4
quit
