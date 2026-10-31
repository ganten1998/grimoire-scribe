# settings: status_mode=1
# The editor's Paper sheet > Photos... > a photo from grimoire/photos: the page goes onto it as onto
# any paper (one undo step), a document page on the photo's own picture, made in the background,
# upright (IMG_0001.jpg is stored sideways with EXIF orientation 6: its upright view has a black block
# at its top left), and drawn once it is there. Undo puts the ruled paper back; a page added after it
# gets the notebook's paper, not the photo.
# copy: tests/fixtures/photos photos
# rust-only
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 400
expect screen is editor
expect editor.paper is 5
pentap panel paper
wait 200
expect editor.modal is paper
pentap panel paper.photos
wait 300
expect editor.modal is templates
expect templates.count is 2
waitbg
wait 300
expect templates.ready is 2
shot 01-photos
pentap 232 452
wait 300
expect editor.modal is none
expect annotate.role is document
waitbg
wait 300
expect editor.picture is shown
expect editor.live_matches_render is 1
shot 02-on-the-photo
pen line 300 1700 1500 1760 40 0.7 0.7
wait 300
expect editor.strokes is 1
expect editor.live_matches_render is 1
pentap panel undo
wait 200
pentap panel undo
wait 300
expect editor.paper is 5
expect editor.strokes is 0
pentap panel redo
wait 300
expect annotate.role is document
pentap panel next
wait 400
expect editor.pages is 2
expect editor.page is 2
expect editor.paper is 5
pentap panel library
wait 600
quit
