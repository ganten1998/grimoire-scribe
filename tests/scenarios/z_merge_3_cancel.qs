# The Merge dialog's third row, Cancel and Esc: nothing changes, not a folder or a mirror PDF.
# settings: shortcuts_enabled=1
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
tap library done
wait 100
expect library.sheet is none
expect library.folders is Archive, Archive/School, Archive/School/Deep, School
expect library.mirror is . 0, Archive_School 2, Archive_School_Deep 1, School 1
tap library folder-menu.Archive/School
wait 100
tap library fmore.move
wait 100
tap library move.top
wait 300
expect library.sheet is merge
key 1 0
wait 100
expect library.sheet is none
expect library.folders is Archive, Archive/School, Archive/School/Deep, School
expect library.mirror is . 0, Archive_School 2, Archive_School_Deep 1, School 1
quit
