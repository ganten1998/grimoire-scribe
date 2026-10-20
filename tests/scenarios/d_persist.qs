# Run after b_library on the same root: everything must come back.
# root: b_library
wait 300
shot 01-library-reopened
expect screen is library
expect library.items is 2
expect settings.pen.valid is 1
expect settings.touch.valid is 1
expect toast is
tap 326 608
wait 300
shot 02-folder
expect library.folder is School
expect library.items is 1
tap 326 608
wait 500
shot 03-notebook-reopened
expect screen is editor
expect editor.page is 1
expect editor.pages is 1
expect editor.paper is 3
expect editor.strokes is 2
expect editor.tool is ballpoint
wait 1700
quit
