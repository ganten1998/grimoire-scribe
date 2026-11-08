# rust-only
# copy: tests/fixtures/merge-tree .
# env: GRIMOIRE_CHECK_FIT=1
# Renaming a folder onto the name of one that is there opens the same Merge dialog (the Rename folder keyboard
# first): Archive, with School inside it, becomes School; Merge into it puts what Archive held into School.
wait 300
tap library folder-menu.Archive
wait 100
expect library.sheet is folder-more
tap library fmore.rename
wait 100
expect library.sheet is keyboard
expect library.text is Archive
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
expect library.text is
tap 175 2205
tap 398 2045
tap 747 2205
tap 1106 2045
tap 1549 1885
tap 1549 1885
tap 1637 2045
expect library.text is School
tap 1643 2365
wait 300
expect library.sheet is merge
expect library.folders is Archive, Archive/School, Archive/School/Deep, School
shot 01-merge-dialog-rename
tap library merge.into
wait 300
expect library.sheet is none
expect library.folders is School, School/School, School/School/Deep
expect library.mirror is . 0, School 1, School_School 2, School_School_Deep 1
expect library.items is 1
quit
