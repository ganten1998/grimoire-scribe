# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_trash_tree.py
# Rename on a template's card in Files (2.2, issue #144): the card of a file inside Grimoire's own folders offers
# Rename and Delete; Rename opens the keyboard with the name, its caret before the extension. A name each rule
# refuses says why and keeps the keyboard up with it (a character a name cannot hold, a space at the start and at
# the end, a dot at the end and at the start, the extension gone, then changed); a name taken says so and nothing
# is replaced; an empty name and Cancel rename nothing; a good name renames the file in place.
# Keys (portrait, the Library's keyboard): shift 175 2205, del 1679 2205, Cancel 217 2365, the symbols key
# 570 2365, space 1018 2365, the dot 1466 2365, Done 1685 2365; letters row y 1885 (q 133, e 487, i 1372, p 1726),
# y 2045 (a 221, s 398, g 929, j 1283, l 1637), y 2205 (n 1278); digits y 1725 (2 at 310); the symbols page's
# colon 664 2045. Hardware keys: Home 102, End 107, Left 105, Ctrl+A 30 with Ctrl (1).
# rust-only
wait 300
tap library files-card
wait 300
tap library files.item.Templates
wait 300
expect files.place is grimoire/templates
expect files.order is Grid.png|Lines.png
tap library files.item.Grid.png
wait 300
expect files.card is picture
expect files.card.rows is Rename|Delete (moves it to grimoire/trash)|Cancel
shot 01-card
tap library files.card.rename
wait 300
expect files.keyboard is 1
expect files.text is Grid.png
expect files.card is
shot 02-keyboard
# a character a name cannot hold, typed where the caret is (before .png)
tap 570 2365
tap 664 2045
tap 570 2365
expect files.text is Grid:.png
tap 1685 2365
wait 200
expect toast is Name can't contain :
expect files.keyboard is 1
expect files.text is Grid:.png
shot 03-refused
tap 1679 2205
expect files.text is Grid.png
# a space at the start, then at the end
key 102 0
tap 1018 2365
tap 1685 2365
wait 200
expect toast is Name can't start or end with a space.
expect files.keyboard is 1
tap 1679 2205
expect files.text is Grid.png
key 107 0
tap 1018 2365
tap 1685 2365
wait 200
expect toast is Name can't start or end with a space.
tap 1679 2205
expect files.text is Grid.png
# a dot at the end, then at the start
tap 1466 2365
expect files.text is Grid.png.
tap 1685 2365
wait 200
expect toast is Name can't start or end with a dot.
tap 1679 2205
key 102 0
tap 1466 2365
expect files.text is .Grid.png
tap 1685 2365
wait 200
expect toast is Name can't start or end with a dot.
tap 1679 2205
expect files.text is Grid.png
# the extension gone, then another one
key 107 0
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
expect files.text is Grid
tap 1685 2365
wait 200
expect toast is Keep the extension .png.
expect files.keyboard is 1
tap 1466 2365
tap 1283 2045
tap 1726 1885
tap 929 2045
expect files.text is Grid.jpg
tap 1685 2365
wait 200
expect toast is Keep the extension .png.
expect storage.names.grimoire/templates is Grid.png|Lines.png|README.txt
# a name taken: nothing replaced
key 30 1
tap 175 2205
tap 1637 2045
tap 1372 1885
tap 1278 2205
tap 487 1885
tap 398 2045
tap 1466 2365
tap 1726 1885
tap 1278 2205
tap 929 2045
expect files.text is Lines.png
tap 1685 2365
wait 200
expect toast is A file with that name is already here.
expect files.keyboard is 1
expect storage.size.grimoire/templates/Lines.png is 2048
# a good name: Lines2.png, the file renamed in place and listed by it
key 105 0
key 105 0
key 105 0
key 105 0
tap 310 1725
expect files.text is Lines2.png
tap 1685 2365
wait 300
expect files.keyboard is 0
expect storage.names.grimoire/templates is Lines.png|Lines2.png|README.txt
expect storage.size.grimoire/templates/Lines2.png is 1024
expect storage.size.grimoire/templates/Lines.png is 2048
expect files.item.Lines2.png is Lines2.png
expect files.item.Grid.png is
shot 04-renamed
# an empty name, and Cancel: nothing renamed
tap library files.item.Lines2.png
wait 300
tap library files.card.rename
wait 300
key 30 1
tap 1679 2205
expect files.text is
tap 1685 2365
wait 300
expect files.keyboard is 0
tap library files.item.Lines2.png
wait 300
tap library files.card.rename
wait 300
tap 221 2045
tap 217 2365
wait 300
expect files.keyboard is 0
expect storage.names.grimoire/templates is Lines.png|Lines2.png|README.txt
quit
