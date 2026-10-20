# What drew a template white while its picture was being made, other than the open page itself:
# here the paper erase strokes restore toward, drawn white by a precise erase on the template page.
# The picture comes while another page is open, or while the page is back on its old paper (the
# template undone), and the cache goes all the same: back on the template, the precise eraser
# restores the picture live, as the saved page shows it. (The jobs are slowed here, as a big
# picture is on the Kindle; the waits hold for them on a busy machine.)
# copy: tests/fixtures/scen-templates templates
# settings: eraser_mode=1
# env: GRIMOIRE_BG_DELAY_MS=3000
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 400
expect screen is editor
pentap 1768 1114
wait 400
pentap 1768 938
wait 400
expect editor.page is 1
pentap 1768 1242
wait 200
pentap 232 1750
wait 10000
expect templates.ready is 2
# Page 1 onto Lined, erased over while its picture is still to come (an erase over blank paper
# changes nothing and is not kept, but it draws the paper cache, white).
pentap 511 452
wait 300
expect editor.paper_hex is 0x50f4d83c
expect editor.picture is missing
eraser line 790 560 810 780 15
wait 500
expect editor.paper_cache is 1
expect editor.erase_strokes is 0
# To page 2 (Medium rule) for the picture's job; the cache goes once it is done.
pentap 1768 1114
wait 400
expect editor.page is 2
wait 10000
expect editor.paper_cache is 0
pentap 1768 938
wait 400
expect editor.page is 1
expect editor.picture is shown
pen line 300 640 1300 700 40 0.6 0.6
wait 300
eraser line 1000 560 1020 780 15
wait 500
expect editor.erase_strokes is 1
expect editor.live_matches_render is 1
shot 01-erased-on-lined-after-another-page
# Page 2 onto Dots, erased over while its picture is still to come; Dots undone (the erase stroke
# was not kept: the paper is the step undone), its picture made meanwhile, then redone.
pentap 1768 1114
wait 400
expect editor.page is 2
pentap 1768 1242
wait 200
pentap 232 1750
wait 300
expect editor.modal is templates
pentap 232 452
wait 300
expect editor.paper_hex not 0x50f4d83c
expect editor.paper_hex has 0x50
expect editor.picture is missing
eraser line 790 560 810 780 15
wait 500
pentap 1768 594
wait 300
expect editor.paper is 1
wait 10000
expect editor.paper_cache is 0
pentap 1768 702
wait 300
expect editor.paper_hex has 0x50
expect editor.picture is shown
pen line 300 640 1300 700 40 0.6 0.6
wait 300
eraser line 1000 560 1020 780 15
wait 500
expect editor.erase_strokes is 1
expect editor.live_matches_render is 1
shot 02-erased-on-dots-after-undo-and-redo
quit
