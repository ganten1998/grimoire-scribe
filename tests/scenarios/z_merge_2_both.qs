# The Merge dialog's second row, Keep both: Archive/School comes to the top level as School 2, beside School; nothing merges and every mirror PDF follows.
# rust-only
# copy: tests/fixtures/merge-tree .
# env: GRIMOIRE_CHECK_FIT=1
wait 300
expect library.folders is Archive, Archive/School, Archive/School/Deep, School
expect library.mirror is . 0, Archive_School 2, Archive_School_Deep 1, School 1
tap library folder.Archive
wait 300
expect library.folder is Archive
tap library folder-menu.Archive/School
wait 100
expect library.sheet is folder-more
tap library fmore.move
wait 100
expect library.sheet is move
tap library move.top
wait 300
expect library.sheet is merge
expect toast is
expect library.ui_at.930.789 is merge.into
expect library.ui_at.930.949 is merge.both
shot 01-merge-dialog
tap library merge.both
wait 300
expect library.sheet is none
expect toast is
expect library.folders is School, School 2, School 2/Deep
expect library.mirror is . 0, School 1, School 2 2, School 2_Deep 1
tap 125 95
wait 300
expect library.items is 2
shot 02-kept-both
quit
