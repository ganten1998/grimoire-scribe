# copy: tests/fixtures/pages-many/notebooks notebooks
# The number grid, and the Find sheet's Go to page, must not load any pages except the one explicitly opened.
# rust-only
wait 200
tap 200 400
wait 200
expect screen is editor
expect editor.pages is 120
expect editor.loaded_pages is 1
pentap panel pages
wait 200
expect editor.modal is pages
expect editor.loaded_pages is 1
shot many-pages-instant-grid
tap 1700 2380
wait 100
expect editor.loaded_pages is 1
tap 930 2380
wait 100
# Go to page... is the Find sheet: 1, 2, 0 on its digits row, then Done on the row "Go to page 120".
expect editor.modal is find
tap 133 1725
tap 310 1725
tap 1726 1725
expect editor.find.1 is Go to page 120
expect editor.loaded_pages is 1
tap 1643 2365
wait 200
expect editor.page is 120
expect editor.loaded_pages is 2
expect editor.modal is none
quit
