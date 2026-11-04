# rust-only
# root: z_lasso_17_clipboard_session
# settings: mirror_pdf=0
# copy: tests/fixtures/clipboard-earlier
# The lasso's clipboard lasts one session: the next start deletes it before anything can paste, so Select mode
# offers no Paste and a hold that would paste does nothing. z_lasso_17 left its clipboard.qpg in this root, and the
# fixture puts one there again after that session's exit, so only the start (app_init) can have deleted it.
wait 300
expect editor.clipboard is 0
tap 200 400
wait 400
expect screen is editor
expect editor.strokes is 1
tap panel tool-picker
tap panel picker.select
wait 100
expect editor.select is armed
expect editor.at.select.paste is
# the pen held still on empty page in Select mode pastes the clipboard: there is none, so nothing
pen down 900 1800
wait 900
pen up 900 1800
wait 200
expect editor.strokes is 1
expect editor.clipboard is 0
quit
