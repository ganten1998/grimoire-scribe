# settings: status_mode=1
# env: GRIMOIRE_CHECK_FIT=1
# settings: theme=classic
# j_pages_card.qs's bottom row, held-page card and Rename keyboard in the Classic theme, upright and
# sideways (shots to look at); the card's Cancel and the keyboard's Cancel each give the grid back.
# rust-only
wait 200
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
expect settings.theme is classic
pen curve 300 500 40 60 0.7 1
pentap panel next
wait 200
pen curve 300 500 40 60 0.7 2
wait 200
expect editor.pages is 2
pentap panel pages
wait 400
expect editor.modal is pages
shot 01-bottom-row
touch down 318 551
wait 700
touch up 318 551
expect editor.pages.context is 1
shot 02-card
tap panel pages.rename
wait 200
expect editor.keyboard is Page name
shot 03-rename-keyboard
tap 125 2365
wait 200
expect editor.keyboard is
expect editor.modal is pages
rotate 1
wait 400
shot 04-sideways-bottom-row
touch down 421 448
wait 700
touch up 421 448
expect editor.pages.context is 1
shot 05-sideways-card
tap panel pages.cancel
wait 200
expect editor.pages.context is 0
expect editor.modal is pages
expect editor.pages is 2
quit
