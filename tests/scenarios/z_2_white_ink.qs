# White ink (white=1, set by hand until the tool picker has Black / White) on the page
# z_1_precise_eraser left: every drawing tool draws white with its own settings and texture (white
# pencil keeps the paper's tooth, white charcoal its grain), the paper's lines stay, live equals a
# fresh render, undo and redo work on it as on ink. White over blank paper changes nothing and is
# not kept, and the highlighter on the pen button (side_button=2) stays ink.
# root: z_1_precise_eraser
# settings: white=1
# settings: side_button=2
wait 300
expect settings.white is 1
expect settings.side_button is 2
tap 326 608
wait 500
expect screen is editor
expect editor.page is 1
expect editor.strokes is 9
expect editor.tool is pencil
# White pencil along the ballpoint underline: the pencil's texture and pen feel, in white
pen line 250 980 1350 1020 80 0.9 0.9
wait 700
shot 01-white-pencil
expect editor.strokes is 10
expect editor.erase_strokes is 6
expect editor.stroke_look is white pencil
expect editor.stroke_settings is pencil
expect editor.live_matches_render is 1
# White charcoal (picked in the tool picker) through the first line of writing
pentap 1768 358
wait 200
pentap 1504 395
wait 200
expect editor.tool is charcoal
pen line 250 700 800 720 60 1.0 1.0
wait 700
shot 02-white-charcoal
expect editor.strokes is 11
expect editor.erase_strokes is 7
expect editor.stroke_look is white charcoal
expect editor.stroke_settings is charcoal
expect editor.live_matches_render is 1
# White over blank paper: nothing changes, nothing is kept
pen line 300 2000 900 2050 40 0.8 0.8
wait 300
expect editor.strokes is 11
# The pen button's highlighter is ink, white or not
side on
pen line 280 1230 1300 1240 60 0.5 0.5
side off
wait 700
shot 03-highlighter-stays-ink
expect editor.strokes is 12
expect editor.erase_strokes is 7
expect editor.stroke_look is highlighter
expect editor.stroke_settings is highlighter
expect editor.live_matches_render is 1
# Undo the highlighter and the white charcoal, then redo the white charcoal
pentap 1768 594
wait 200
pentap 1768 594
wait 300
expect editor.strokes is 10
expect editor.live_matches_render is 1
pentap 1768 702
wait 700
shot 04-redo-white
expect editor.strokes is 11
expect editor.erase_strokes is 7
expect editor.stroke_look is white charcoal
expect editor.live_matches_render is 1
pentap 1768 1566
wait 1500
expect screen is library
expect bg.busy is 0
quit
