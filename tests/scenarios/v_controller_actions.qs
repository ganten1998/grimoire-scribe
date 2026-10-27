# settings: shortcuts_enabled=1
# copy: tests/fixtures/controller-actions .
# The controller's actions in the editor (controller.ini gives each a button): a single pen and the
# eraser, the menus (each button opens its own menu and closes it again, swaps the one in front for
# its own, Close menu closes any; nothing else acts through one, next tool neither, but the picker's
# next and previous move the open tool picker's choice), the panel folded, the page
# cleared, pages added, first and last, the whole page again after a pinch, the eraser's mode, and
# Erase while held (from its press to its release, or the controller going).
wait 150
expect pads.hold-eraser is L2
expect pads.close-menu is 613
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
key 614 0 1
wait 60
expect editor.tool is fountain
key 305 0 1
wait 60
expect editor.tool is eraser
key 305 0 1
wait 60
expect editor.tool is fountain
# Menus
key 608 0 1
wait 100
expect editor.modal is paper
key 608 0 1
wait 100
expect editor.modal is none
key 609 0 1
wait 100
expect editor.modal is display
key 607 0 1
wait 100
expect editor.modal is tools
key 311 0 1
wait 60
expect editor.tool is fountain
expect editor.modal is tools
key 616 0 1
wait 60
expect editor.tool is calligraphy
expect editor.modal is tools
key 615 0 1
wait 60
expect editor.tool is fountain
key 616 0 1
wait 60
expect editor.tool is calligraphy
key 614 0 1
wait 60
expect editor.tool is calligraphy
key 607 0 1
wait 100
expect editor.modal is none
key 605 0 1
wait 200
expect editor.modal is pages
expect editor.pages_tab is pages
key 606 0 1
wait 200
expect editor.modal is pages
expect editor.pages_tab is index
key 605 0 1
wait 200
expect editor.pages_tab is pages
key 608 0 1
wait 200
expect editor.modal is paper
key 613 0 1
wait 100
expect editor.modal is none
key 613 0 1
wait 100
expect editor.modal is none
# The panel folded and unfolded
key 611 0 1
wait 100
expect editor.panel is collapsed
key 611 0 1
wait 100
expect editor.panel is expanded
# Clear page, and undo brings the ink back
pen line 300 700 900 720 10 0.6 0.7
pen line 300 900 900 920 10 0.6 0.7
wait 100
expect editor.strokes is 2
key 600 0 1
wait 100
expect editor.strokes is 0
expect toast is Page cleared
key 304 0 1
wait 100
expect editor.strokes is 2
# New page (after the open one), last and first
key 603 0 1
wait 200
expect editor.pages is 2
expect editor.page is 2
key 603 0 1
wait 200
expect editor.pages is 3
expect editor.page is 3
key 601 0 1
wait 200
expect editor.page is 1
key 603 0 1
wait 200
expect editor.pages is 4
expect editor.page is 2
key 602 0 1
wait 200
expect editor.page is 4
key 601 0 1
wait 200
expect editor.page is 1
expect editor.strokes is 2
# Fit page after a pinch
pinch 930 1240 400 900 10
wait 200
expect editor.zoomed is 1
key 604 0 1
wait 200
expect editor.zoomed is 0
# The eraser's mode
key 313 0 1
wait 60
expect settings.eraser_mode is 1
expect toast is Eraser: Precise
key 313 0 1
wait 60
expect settings.eraser_mode is 0
expect toast is Eraser: Whole strokes
# Erase while held: the pen erases while L2 is down (whole strokes), draws again once it is let go
key 312 0 1
wait 60
expect editor.hold_eraser is 1
expect editor.tool is calligraphy
pen line 600 600 600 1000 10 0.6 0.7
wait 100
expect editor.strokes is 0
keyup 312 0 1
wait 60
expect editor.hold_eraser is 0
pen line 300 700 900 720 10 0.6 0.7
wait 100
expect editor.strokes is 1
# Precise: an erase stroke; a stroke under way when the button goes keeps erasing
key 313 0 1
key 312 0 1
pen down 600 600 0.6
pen move 600 700 0.6
keyup 312 0 1
pen move 600 800 0.6
pen move 600 900 0.6
pen up 600 900
wait 100
expect editor.strokes is 2
expect editor.erase_strokes is 1
key 313 0 1
wait 60
expect settings.eraser_mode is 0
# The controller going ends the hold too; another button's release does not
hid attach pad Pro Controller
key 312 0 1
keyup 304 0 1
wait 60
expect editor.hold_eraser is 1
hid detach Pro Controller
wait 60
expect editor.hold_eraser is 0
# Nothing else acts through a menu
key 609 0 1
wait 100
expect editor.modal is display
key 603 0 1
wait 100
expect editor.pages is 4
key 613 0 1
wait 100
expect editor.modal is none
# Close menu while a link waits for its box, and while the Pages view picks where a new one goes:
# no link is made (its box is taken back, as Back takes it)
key 606 0 1
wait 200
tap 1410 2395
wait 100
expect editor.link_mode is 1
key 613 0 1
wait 100
expect editor.link_mode is 0
expect toast is No link made.
expect editor.strokes is 2
key 606 0 1
wait 200
tap 1410 2395
wait 100
pen line 300 1000 900 1200 30 0.6 0.6
wait 200
expect editor.picking is new
expect editor.strokes is 3
key 613 0 1
wait 200
expect editor.modal is none
expect toast is No link made.
expect editor.strokes is 2
# Pen feel opens, and its button or Close menu goes back
key 610 0 1
wait 200
expect screen is penset
key 610 0 1
wait 200
expect screen is editor
key 610 0 1
wait 200
key 613 0 1
wait 200
expect screen is editor
# The Library over a menu, with a page it cannot save: it stays, and the menu is gone (the page is
# drawn again where it was)
storage full
pen line 300 1300 900 1320 10 0.6 0.7
wait 100
key 605 0 1
wait 200
expect editor.modal is pages
key 612 0 1
wait 300
expect screen is editor
expect editor.modal is none
expect toast is Could not save this page. Is the storage full?
storage ok
# The Library
key 612 0 1
wait 400
expect screen is library
quit
