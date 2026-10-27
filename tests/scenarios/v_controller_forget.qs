# settings: shortcuts_enabled=1
# A finger held on a Buttons row (0.6 s) takes that row's buttons away, one row at a time: it reads
# none, a toast says so, the release starts no learning, and controller.ini keeps it (shortcuts.ini's
# pad. lines and the D-pad's starter lines stay set aside). A finger that moves off is no hold, and a
# tap still learns.
wait 150
tap 1535 95
wait 200
tap 746 1930
wait 200
tap 930 1215
wait 200
tap 930 682
wait 100
expect library.buttons_group is editing
expect pads.undo is South
expect pads.redo is North
touch down 930 576
wait 300
expect pads.undo is South
wait 400
expect pads.undo is
expect library.buttons.undo is none
expect toast is No buttons for Undo now.
expect library.learning is
shot 01-held
touch up 930 576
wait 100
expect library.learning is
expect pads.redo is North
expect pads.tool:eraser is East
# A tap still learns, the row that was forgotten too
tap 930 576
wait 100
expect library.learning is undo
key 307 0 1
wait 100
expect pads.undo is North
expect pads.redo is
# Pages: the previous / next page row, both actions and the D-pad's Left and Right with them
tap 930 1696
wait 100
tap 930 1002
wait 100
expect library.buttons_group is pages
expect pads.previous-page is Select,Left
touch down 930 523
wait 700
touch up 930 523
wait 100
expect pads.previous-page is
expect pads.next-page is
expect toast is No buttons for Previous / next page now.
expect library.learning is
# A finger that moves off the row: no hold, no learning
touch down 930 683
touch move 930 794
wait 700
touch up 930 794
wait 100
expect library.learning is
# The Tools group's previous / next tool keep the D-pad's Up and Down
expect pads.previous-tool is L1,Up
quit
