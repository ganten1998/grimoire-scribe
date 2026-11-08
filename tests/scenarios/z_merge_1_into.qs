# Moving a folder onto the name of one that is there (z_folder_move's refusal, 2.1): the Merge dialog's first row, Merge into it. Archive/School into the top level's School: everything in it goes in, the notebook that clashes is Notes 2, the mirror PDFs follow, the folder is gone.
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
tap library merge.into
wait 300
expect library.sheet is none
expect toast is
expect library.folders is School, School/Deep
expect library.mirror is . 0, School 3, School_Deep 1
tap 125 95
wait 300
expect library.folder is
tap library folder.School
wait 300
expect library.items is 4
expect library.card.1.name is Deep
expect library.card.2.name is Plan
expect library.card.3.name is Notes 2
expect library.card.4.name is Notes
shot 02-merged
quit
