# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_trash_tree.py
# env: GRIMOIRE_SIM_CLOCK=script
# Two taps at one place on a card's Delete never answer its question (2.2, issue #144): Delete asks first, and the
# question must be seen before it can be answered. The first tap opens `Delete "name"?`; the second, at the place of
# the first (`files.card.opener`: where the card's Delete was) and with no time between, answers nothing, wherever
# the question was drawn: the question is up and the item in its folder. On a folder's card and a file's, each on the
# first row of its folder, upright and sideways. Taps on the question's Delete, on Keep it and outside the sheet a
# tenth of a second after it was shown answer nothing either. Half a second after it was shown (the script's clock
# moves by its waits alone, so the times are exact on any machine) a tap is an answer as before: Keep it keeps, and
# the question's Delete moves the item to the trash.
# rust-only
wait 300
tap library files-card
wait 300
tap library files.item.Photos
wait 300
expect files.order is Other|Trip|beach.jpg
# upright, a folder's card on the first row
tap library files.more.1
wait 300
expect files.card is folder
tap library files.card.trash
tap library files.card.opener
wait 100
expect files.card is delete
expect files.card.details is Delete "Other"?
expect toast is
expect storage.exists.grimoire/photos/Other/q1.png is 1
shot 01-folder-asked
# a tenth of a second after it was shown: no answer yet, on its Delete, on Keep it or outside the sheet
tap library files.card.delete
tap library files.card.cancel
tap 930 60
wait 100
expect files.card is delete
expect toast is
expect storage.exists.grimoire/photos/Other/q1.png is 1
# half a second after it was shown: Keep it
wait 300
tap library files.card.cancel
wait 300
expect files.card is
expect files.order is Other|Trip|beach.jpg
# upright, a file's card on the first row
tap library files.crumb.1
wait 300
tap library files.item.Templates
wait 300
expect files.order is Grid.png|Lines.png
tap library files.item.Grid.png
wait 300
expect files.card is picture
tap library files.card.trash
tap library files.card.opener
wait 300
expect files.card is delete
expect files.card.details is Delete "Grid.png"?
expect toast is
expect storage.exists.grimoire/templates/Grid.png is 1
expect files.order is Grid.png|Lines.png
shot 02-file-asked
# and its Delete, once the question has been shown, moves the file
wait 200
tap library files.card.delete
wait 300
expect toast is Moved to grimoire/trash
expect files.card is
expect storage.exists.grimoire/templates/Grid.png is 0
expect files.order is Lines.png
# sideways, a file's card on the first row
rotate 1
wait 500
expect files.order is Lines.png
tap library files.item.Lines.png
wait 300
expect files.card is picture
tap library files.card.trash
tap library files.card.opener
wait 300
expect files.card is delete
expect files.card.details is Delete "Lines.png"?
expect storage.exists.grimoire/templates/Lines.png is 1
shot 03-file-asked-sideways
wait 200
tap library files.card.delete
wait 300
expect toast is Moved to grimoire/trash
expect storage.exists.grimoire/templates/Lines.png is 0
# sideways, a folder's card on the first row
tap library files.crumb.1
wait 300
tap library files.item.Photos
wait 300
expect files.order is Other|Trip|beach.jpg
tap library files.more.1
wait 300
expect files.card is folder
tap library files.card.trash
tap library files.card.opener
wait 300
expect files.card is delete
expect files.card.details is Delete "Other"?
expect storage.exists.grimoire/photos/Other/q1.png is 1
shot 04-folder-asked-sideways
wait 200
tap library files.card.delete
wait 300
expect toast is Moved to grimoire/trash
expect files.card is
expect files.order is Trip|beach.jpg
expect storage.exists.grimoire/photos/Other/q1.png is 0
quit
