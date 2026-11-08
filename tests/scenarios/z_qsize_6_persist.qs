# root: z_qsize_1_hold
# rust-only
# Quick sizes round-trip through settings.ini: the eraser's 12 mm chip picked in z_qsize_1_hold is what the next
# run reads back (the same two keys every version reads), and the hold's pen pick with it.
wait 600
expect settings.eraser.size_max_mm is 12.0000
expect settings.eraser.size_min_mm is 4.2900
expect settings.pencil.size_max_mm is 0.3200
