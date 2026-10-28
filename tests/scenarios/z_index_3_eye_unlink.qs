# settings: status_mode=1
# The eye (the link areas, on) follows the links the moment they change: a link made shows its dotted box at once, and
# a link removed takes its box off the glass at once (the owner on a Kindle: links that were deleted lingered with the
# show mode on, until the page was swapped out and back). editor.eye_drawn is how many boxes the eye's LAST PAINT drew.
# env: GRIMOIRE_CHECK_FIT=1
# settings: shortcuts_enabled=1
# settings: link_areas=1
# copy: tests/fixtures/index .
# rust-only
wait 100
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
pentap panel next
pentap panel previous
pentap panel previous
expect editor.page is 1
pen line 300 600 900 620 30 0.5 0.9
pentap panel pages
tap 850 95
tap 1410 2395
pen line 300 1000 900 1200 30 0.6 0.6
expect editor.picking is new
tap 1155 340
expect editor.modal is none
expect editor.links is 1 1
expect editor.areas is 1
expect editor.eye_drawn is 1
shot 01-eye-link-made
# removed from the page: the box is off the glass at once, not at the next page swap
touch down 600 1100
wait 800
touch up 600 1100
tap 1630 95
expect editor.links is 0 0
expect toast is Link removed. Its ink stays on the page.
expect editor.eye_drawn is 0
expect editor.areas is 0
shot 02-eye-link-removed
# and made again, and removed again
pentap panel pages
tap 850 95
tap 1410 2395
pen line 300 1400 900 1500 30 0.6 0.6
tap 1155 340
expect editor.eye_drawn is 1
touch down 600 1450
wait 800
touch up 600 1450
tap 1630 95
expect editor.links is 0 0
expect editor.eye_drawn is 0
quit
