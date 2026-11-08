# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: panel_tools=fineliner
# rust-only
# Quick sizes are settings, not page changes: a pick leaves the page, the history and the toolbar's Undo alone.
# Undo after a pick takes back the page's last stroke, never the size; old strokes keep their width, the next one
# has the new.
wait 3600
tap 965 95
wait 200
tap library paper.1
tap 1564 2254
wait 400
expect screen is editor
tap panel tool:fineliner
wait 100
pen line 300 700 1300 720 40 0.7 0.7
wait 400
pen leave
expect editor.strokes is 1
expect editor.stroke_screen_mm is 0.25
expect editor.undo_kind is add
touch down 1768 466
wait 700
touch up 1768 466
wait 200
expect editor.quicksize is fineliner
tap panel quicksize.2
wait 200
expect settings.fineliner.size_max_mm is 0.5000
expect editor.quicksize is 0
expect editor.strokes is 1
expect editor.undo_kind is add
expect editor.stroke_screen_mm is 0.25
# Undo takes the stroke; the size stays.
tap panel undo
wait 300
expect editor.strokes is 0
expect settings.fineliner.size_max_mm is 0.5000
tap panel redo
wait 300
expect editor.strokes is 1
expect editor.stroke_screen_mm is 0.25
# The next stroke has the new width, the one before it keeps its own.
pen line 300 900 1300 920 40 0.7 0.7
wait 400
pen leave
expect editor.strokes is 2
expect editor.stroke_screen_mm is 0.50
tap panel undo
wait 300
expect editor.stroke_screen_mm is 0.25
