# settings: status_mode=1
# settings: shortcuts_enabled=1
# copy: tests/fixtures/photos photos
# Next page (the keyboard's Right, a controller's Start) on the last page adds a page, exactly as the
# editor's own + does there: on the open page's paper, or on the notebook's own after a photo (a
# document picture is never carried over: z_photo_1_page.qs is the panel's twin). Previous page on
# the first page stays.
# rust-only
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 400
expect editor.paper is 5
key 314 0 1
wait 80
expect editor.pages is 1
expect editor.page is 1
key 315 0 1
wait 200
expect editor.pages is 2
expect editor.page is 2
expect editor.paper is 5
# Page 2 onto a photo, then the key on it: a page on the notebook's paper, not the photo's picture
pentap panel paper
wait 200
pentap 511 1750
wait 300
waitbg
wait 300
expect templates.ready is 2
pentap 232 452
wait 300
expect annotate.role is document
key 106 0
wait 400
expect editor.pages is 3
expect editor.page is 3
expect editor.paper is 5
expect annotate.role is
pentap panel library
wait 600
expect screen is library
quit
