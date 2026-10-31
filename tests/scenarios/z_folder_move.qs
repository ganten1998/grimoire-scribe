# root: b_library
# rust-only
# Folders move (Move folder in a folder's menu): with all they hold, never into themselves or a folder
# inside them, back to the top level, into a folder New folder... makes, refused onto a folder of the
# same name; the mirror PDFs follow and the directories left empty go
wait 200
expect library.folder is
expect library.folders is School
expect library.mirror is . 1, School 1
# A folder Math in School, and the top level's notebook into it
tap library folder.School
wait 300
expect library.folder is School
tap 1155 95
wait 100
expect library.sheet is keyboard
tap 175 2205
tap 1455 2205
tap 221 2045
tap 841 1885
tap 1106 2045
expect library.text is Math
tap 1643 2365
wait 300
expect library.folders is School, School/Math
tap 125 95
wait 300
expect library.folder is
tap 1151 1052
wait 200
expect library.sheet is more
tap library more.move
wait 100
expect library.sheet is move
tap library move.to.School/Math
wait 300
expect library.items is 1
expect library.mirror is . 0, School 1, School_Math 1
# A folder Archive at the top level
tap 1155 95
wait 100
tap 175 2205
tap 221 2045
tap 664 1885
tap 747 2205
tap 1106 2045
tap 1372 1885
tap 924 2205
tap 487 1885
expect library.text is Archive
tap 1643 2365
wait 300
expect library.items is 2
# School's menu has Move folder; Move to offers the top level (where it is) and Archive, not School
# or School/Math
tap library folder-menu.School
wait 100
expect library.sheet is folder-more
shot 01-folder-menu
tap library fmore.move
wait 100
expect library.sheet is move
shot 02-move-list
expect library.ui_at.930.789 is move.top
expect library.ui_at.930.949 is move.to.Archive
expect library.ui_at.930.1109 is move.new
expect library.ui_at.930.1269 is done
tap library move.to.Archive
wait 300
expect library.sheet is none
expect toast is
expect library.items is 1
expect library.folders is Archive, Archive/School, Archive/School/Math
expect library.mirror is . 0, Archive_School 1, Archive_School_Math 1
shot 03-moved
# Inside Archive: School, holding its notebook and Math (holding the other)
tap library folder.Archive
wait 300
expect library.folder is Archive
expect library.items is 1
tap library folder.Archive/School
wait 300
expect library.folder is Archive/School
expect library.items is 2
tap library folder.Archive/School/Math
wait 300
expect library.folder is Archive/School/Math
expect library.items is 1
tap 125 95
wait 300
tap 125 95
wait 300
expect library.folder is Archive
# A top-level folder named School: moving Archive/School to the top is refused, with the toast
tap 125 95
wait 300
expect library.folder is
tap 1155 95
wait 100
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
expect library.folders is Archive, Archive/School, Archive/School/Math, School
tap library folder.Archive
wait 300
tap library folder-menu.Archive/School
wait 100
tap library fmore.move
wait 100
expect library.sheet is move
tap library move.top
wait 300
expect toast has Could not move the folder
expect library.sheet is none
expect library.folders is Archive, Archive/School, Archive/School/Math, School
expect library.mirror is . 0, Archive_School 1, Archive_School_Math 1
shot 04-refused
# Without the clash (the empty top-level School removed) it goes back to the top level
tap 125 95
wait 300
tap library folder-menu.School
wait 100
tap library fmore.remove
wait 300
expect library.folders is Archive, Archive/School, Archive/School/Math
tap library folder.Archive
wait 300
tap library folder-menu.Archive/School
wait 100
tap library fmore.move
wait 100
tap library move.top
wait 300
expect library.items is 0
expect library.folders is Archive, School, School/Math
expect library.mirror is . 0, School 1, School_Math 1
tap 125 95
wait 300
expect library.items is 2
# New folder... in Move to makes a folder beside it and moves it in
tap library folder-menu.School
wait 100
tap library fmore.move
wait 100
tap library move.new
wait 100
expect library.sheet is keyboard
tap 175 2205
tap 1549 1885
tap 1637 2045
tap 575 2045
expect library.text is Old
tap 1643 2365
wait 300
expect library.sheet is none
expect library.folders is Archive, Old, Old/School, Old/School/Math
expect library.mirror is . 0, Old_School 1, Old_School_Math 1
tap library folder.Old
wait 300
tap library folder.Old/School
wait 300
expect library.folder is Old/School
expect library.items is 2
shot 05-new-folder-move
quit
