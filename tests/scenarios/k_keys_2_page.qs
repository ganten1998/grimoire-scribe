# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# rust-only
# The keyboard scheme on a notebook page, on a keyboard
# paired with the Kindle (the starter map): 0 and E the eraser, - and = (+) zoom, Up and Down nothing, PageUp and
# PageDown turn pages, Ctrl+L Add a link, R the page's name, Space what two fingers up do (the Links bar), Ctrl+S
# saves, and Esc one layer a press: a keyboard's Cancel, a menu, the lasso's selection then Select mode, the
# Pages view, a link being placed, then back to the Library.
wait 200
# Ctrl+N in the Library, Enter: New notebook's Create
key 49 1
wait 100
expect library.sheet is new
key 28 0
wait 300
expect screen is editor
expect editor.pages is 1
# 0 and E: the eraser; 1 the pencil
key 11 0
wait 100
expect editor.tool is eraser
key 2 0
wait 100
expect editor.tool is pencil
key 18 0
wait 100
expect editor.tool is eraser
key 2 0
wait 100
# = (and Shift+=, the keypad's +) zoom in at the centre, - (the keypad's -) out; Up and Down move nothing
key 13 0
wait 100
expect editor.zoom is 1.25
key 13 2
wait 100
expect editor.zoom is 1.56
expect editor.view is 1.56 335 446
key 103 0
key 108 0
wait 100
expect editor.zoom is 1.56
expect editor.view is 1.56 335 446
key 74 0
key 12 0
wait 100
expect editor.zoom is 1.00
key 78 0
key 12 0
wait 100
expect editor.zoom is 1.00
# Ctrl+N a page after this one; PageUp and PageDown turn pages
key 49 1
wait 300
expect editor.pages is 2
expect editor.page is 2
key 104 0
wait 300
expect editor.page is 1
key 109 0
wait 300
expect editor.page is 2
key 105 0
wait 300
expect editor.page is 1
# Ctrl+L: Add a link (the strip, the next stroke its box); Esc: not made
key 38 1
wait 100
expect editor.link_mode is 1
key 1 0
wait 100
expect editor.link_mode is 0
# R: the page's name keyboard; Esc is its Cancel, back to the page
key 19 0
wait 300
expect editor.keyboard is Page name
key 1 0
wait 300
expect editor.modal is none
# Space: what two fingers up do (built in: the Links bar); Esc closes it
key 57 0
wait 300
expect editor.bar is open
key 1 0
wait 300
expect editor.bar is 0
expect editor.modal is none
# T: the tool picker; Esc closes it
key 20 0
wait 200
expect editor.modal is tools
key 1 0
wait 200
expect editor.modal is none
# The lasso: a selection, then Esc drops it (Select mode stays), Esc again ends Select mode
pen line 500 700 800 720 20 0.5 0.7
wait 300
key 31 0
wait 100
expect editor.select is armed
pen down 350 600
pen move 950 600
pen move 950 850
pen move 350 850
pen move 350 620
pen up 350 620
wait 300
expect editor.select is selected
key 1 0
wait 100
expect editor.select is armed
key 1 0
wait 100
expect editor.select is off
expect screen is editor
expect editor.strokes is 1
# Ctrl+S: written now, and says so
expect editor.unsaved is 1
key 31 1
wait 100
expect editor.unsaved is 0
expect toast is Saved
# P: the Pages view; Esc closes it
key 25 0
wait 300
expect editor.modal is pages
key 1 0
wait 300
expect editor.modal is none
# Esc with nothing above the page: back to the Library, saved
key 1 0
wait 400
expect screen is library
expect library.items is 1
