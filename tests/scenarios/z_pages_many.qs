# copy: tests/fixtures/pages-many/notebooks notebooks
# The number grid must not load any pages except the one explicitly opened.
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
# Number pad: 1, 2, 0, Done.
tap 343 1725
tap 945 1725
tap 945 2223
tap 930 2389
wait 200
expect editor.page is 120
expect editor.loaded_pages is 2
expect editor.modal is none
quit
