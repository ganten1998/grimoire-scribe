# rust-only
# copy: tests/fixtures/subfolder-tree .
# env: GRIMOIRE_CHECK_FIT=1
wait 300
expect library.items is 2
expect library.card.1.name is A
expect library.card.2.name is AB
tap 326 608
expect library.folder is A
expect library.items is 2
expect library.card.1.name is B
expect library.card.2.name is Empty
shot 01-inside-A
tap 326 608
expect library.folder is A/B
expect library.items is 1
expect library.card.1.name is C
# Folder creation works at the current depth.
tap 1155 95
expect library.sheet is keyboard
tap 221 2045
expect library.text is a
tap 1643 2365
expect library.folder is A/B
expect library.items is 2
shot 02-created-child
tap 125 95
expect library.folder is A
tap 125 95
expect library.folder is
expect library.items is 2
