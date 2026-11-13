# settings: status_mode=1
# rust-only
# The caret and the keyboard's clipboard on the Find sheet (2.1.0-beta.2, issue #36): its rows follow the text,
# so a letter typed at the caret, a cut and a paste make them again, and a move of the caret does not (the text
# line and the key are all it refreshes). A paired keyboard's Right moves the caret and turns no page under the
# sheet. The keyboard's clipboard is apart from the lasso's: ink copied stays the lasso's to paste, text copied
# the keyboard's.
# root: z_bar_0_make
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3700
tap 200 400
wait 300
# page 2: a stroke, lassoed and copied (the lasso's clipboard)
pentap panel next
wait 300
expect editor.page is 2
expect editor.strokes is 0
pentap panel tool-picker
pentap 872 561
wait 100
pen line 400 1100 800 1120 30 0.6
wait 100
expect editor.strokes is 1
tap panel tool-picker
tap panel picker.select
wait 100
pen down 300 1000
pen move 900 1000
pen move 900 1220
pen move 300 1220
pen up 300 1010
wait 100
expect editor.selected is 1
tap panel select.copy
wait 100
expect editor.clipboard is 1
tap panel select.done
wait 100
# Find: "bl" matches no name
key 34 0
wait 300
expect editor.find is go
tap 1101 2205
tap 1637 2045
expect editor.caret is bl|
expect editor.find.1 is No page names match.
wait 1200
# The left arrow: the text line and the key, no rows
expect display.presents not -1
tap 1549 1570
expect editor.caret is b|l
expect display.presents is fast 50 1510 1760 120, fast 1466 1510 167 120
expect editor.find.1 is No page names match.
# o typed at the caret: "bol" is in Arbol
tap 1549 1885
expect editor.caret is bo|l
expect editor.find.rows is 1
expect editor.find.1 is Árbol
# A paired keyboard's Right: the caret, not the page
key 106 0
expect editor.caret is bol|
expect editor.page is 2
expect editor.find is go
# Ctrl+A, Ctrl+X: the text gone, the rows with it; Ctrl+V: back
key 30 1
expect editor.caret is [bol|]
key 45 1
expect editor.text is
expect editor.find.rows is 0
expect keyboard.clipboard is bol
expect editor.clipboard is 1
key 47 1
expect editor.caret is bol|
expect editor.find.1 is Árbol
expect editor.strokes is 1
# The go-to-page key closes Find; the page's Name this page (N in this root's shortcuts) takes the same
# clipboard, and the same keys
key 34 0
wait 300
expect editor.find is 0
key 49 0
wait 300
expect editor.keyboard is Page name
key 47 1
expect editor.caret is bol|
key 102 2
expect editor.caret is [|bol]
key 106 0
expect editor.caret is bol|
key 105 0
expect editor.caret is bo|l
tap 175 2365
wait 300
expect editor.keyboard is
# Select mode's Paste still pastes the ink
tap panel tool-picker
tap panel picker.select
wait 100
tap panel select.paste
wait 100
expect editor.strokes is 2
expect keyboard.clipboard is bol
quit
