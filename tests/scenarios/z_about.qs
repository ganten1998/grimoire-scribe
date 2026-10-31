# settings: status_mode=1
# What's new: the bat (or the name beside it) opens the card; Don't show this again is a setting,
# kept at once; Read all shows the whole notes on the sheet, a page at a time sideways; Close.
# rust-only
wait 200
expect screen is library
expect library.sheet is none
expect library.ui_at.120.95 is about
expect library.ui_at.400.95 is about
tap 120 95
wait 100
expect library.sheet is about
expect library.about.full is 0
shot 01-card
tap library about.hide
wait 50
expect settings.whatsnew_hide is 1
shot 02-ticked
tap library about.hide
wait 50
expect settings.whatsnew_hide is 0
tap library about.more
wait 100
expect library.about.full is 1
expect library.about.pages is 1
shot 03-all
rotate 1
wait 200
expect library.sheet is about
expect library.about.pages is 2
expect library.about.page is 1
shot 04-all-sideways
tap library about.next
wait 50
expect library.about.page is 2
shot 05-all-sideways-2
tap library about.prev
wait 50
expect library.about.page is 1
rotate 0
wait 200
tap library about.close
wait 100
expect library.sheet is none
tap 400 95
wait 100
expect library.sheet is about
expect library.about.full is 0
tap library about.close
wait 100
expect library.sheet is none
quit
