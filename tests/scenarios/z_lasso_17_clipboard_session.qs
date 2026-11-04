# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# The lasso's clipboard lasts one session (the owner, 2026-11-04: "yes the lasso clipboard should clear on
# restart"). A copy made in this session pastes after a page turn and in another notebook; the clipboard file
# is still there when the session ends (z_lasso_18_clipboard_restart starts from this root and finds it gone).
wait 300
expect editor.clipboard is 0
tap 965 95
wait 100
tap 1564 2254
wait 400
expect screen is editor
pentap panel tool-picker
pentap 872 561
wait 100
pen line 400 1100 800 1120 30 0.6
wait 100
expect editor.strokes is 1
tap panel tool-picker
tap panel picker.select
wait 100
expect editor.select is armed
# no clipboard yet: Select mode offers no Paste
expect editor.at.select.paste is
pen down 300 1000
pen move 900 1000
pen move 900 1220
pen move 300 1220
pen up 300 1010
wait 100
expect editor.select is selected
expect editor.selected is 1
tap panel select.copy
wait 100
expect editor.clipboard is 1
tap panel select.done
wait 100
# a page turn (the + on the last page: a new page 2), then Paste there
pentap panel next
wait 300
expect editor.page is 2
expect editor.strokes is 0
tap panel tool-picker
tap panel picker.select
wait 100
tap panel select.paste
wait 100
expect editor.strokes is 1
expect editor.clipboard is 1
tap panel select.done
wait 100
# back a page and forward again: the clipboard is still there
pentap panel previous
wait 300
pentap panel next
wait 300
expect editor.page is 2
expect editor.clipboard is 1
# another notebook: it pastes there too
pentap panel library
wait 500
expect screen is library
tap 965 95
wait 100
tap 1564 2254
wait 400
expect screen is editor
expect editor.strokes is 0
expect editor.clipboard is 1
tap panel tool-picker
tap panel picker.select
wait 100
tap panel select.paste
wait 100
expect editor.strokes is 1
tap panel select.done
wait 100
pentap panel library
wait 500
expect screen is library
expect editor.clipboard is 1
quit
