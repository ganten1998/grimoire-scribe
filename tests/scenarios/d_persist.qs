# Run after b_library on the same root: everything must come back.
wait 300
shot 01-library-reopened
tap 326 608
wait 300
shot 02-folder
tap 326 608
wait 500
shot 03-notebook-reopened
wait 1700
quit
