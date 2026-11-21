# settings: status_mode=1
# The Templates sheet's From a notebook... (#195): pick a notebook and one of its pages, and that page is drawn as
# the open page's template in the background. The Templates sheet goes, the page takes the picture's paper at once
# (white until the child has made it), and the picture is drawn when the job ends. Over New notebook, the page
# picked is the page the notebook it creates opens on.
# copy: tests/fixtures/scen-tpl-restore .
# rust-only
wait 300
expect screen is library
tap 326 608
wait 600
expect screen is editor
expect editor.page is 2
pentap panel paper
wait 200
expect editor.modal is paper
pentap panel paper.more
wait 300
expect editor.modal is templates
tap panel templates.from_notebook
wait 300
expect editor.destination is 1
shot 01-pick-the-notebook
tap panel elsewhere.current
wait 300
expect editor.destination.notebook is 20261020-000000-0004
shot 02-pick-the-page
tap panel elsewhere.page.1
wait 300
expect editor.modal is none
expect editor.paper_hex has 0x50
waitbg
wait 300
expect editor.picture is shown
expect editor.live_matches_render is 1
shot 03-on-the-page
# Its Paper sheet still opens, and the Templates sheet over it, with the page's paper on a picture (not a
# template file: the folder's From a file... tile is still there).
pentap panel paper
wait 200
pentap panel paper.more
wait 300
expect editor.modal is templates
pentap 300 1835
wait 200
expect editor.modal is paper
tap 1562 2125
wait 200
expect editor.modal is none
# New notebook's Templates sheet offers it too: the notebook it creates opens on the page picked.
pentap panel library
wait 600
expect screen is library
tap 965 95
wait 200
tap library paper.more
wait 300
expect library.sheet is templates
tap library templates.from_notebook
wait 300
expect library.picker is 1
shot 04-new-pick-the-notebook
tap library elsewhere.book.20261020-000000-0004
wait 300
shot 05-new-pick-the-page
tap library elsewhere.page.1
wait 300
expect library.picker is 0
expect library.paper is notebook
tap library create
wait 1000
expect screen is editor
expect editor.paper_hex has 0x50
waitbg
wait 300
expect editor.picture is shown
shot 06-new-notebook-on-the-page
quit
