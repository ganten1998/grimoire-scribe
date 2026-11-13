# settings: status_mode=1
# settings: status_on=0
# settings: status_corner=3
# env: GRIMOIRE_IMPORT_STABLE=fast GRIMOIRE_PDF_SLICE_PAGES=2
# copy: tests/fixtures/pdf/tabs import
# pdfs: 0
# The clock corner (bottom left here) and a document link under it (2.1.0-beta.2, D7a: taps follow document links;
# the corner's tap is the clock's over a link the owner drew, but a planner's tab is not that): the tab that lies
# under the corner takes the tap and goes to its page; where no link lies, the corner's tap is still the clock's.
# rust-only
waitbg
wait 300
expect library.items is 1
tap 326 608
wait 700
expect screen is editor
expect editor.page is 1
expect settings.status_on is 0
# a tap in the corner where no link lies toggles the clock
tap 20 2460
wait 300
expect settings.status_on is 1
expect editor.page is 1
tap 20 2460
wait 300
expect settings.status_on is 0
# the named link's box (80 2240 320 160) holds a part of the corner: its tap follows the link, the clock stays off
tap 120 2360
wait 400
expect editor.page is 4
expect settings.status_on is 0
pentap panel library
wait 600
expect screen is library
quit
