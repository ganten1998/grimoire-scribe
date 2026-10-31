# rust-only
# generate: tests/page_management_fixture.py
# env: GRIMOIRE_CHECK_FIT=1
wait 300
tap library folder.Void
tap library notebook.A
swipe 2 930 900 0 400
expect editor.map.visible is 1 2
expect editor.map.foreign is 1
expect editor.map.stats is 3 13 3 15 20
expect editor.screen_composed is 1
shot 01-compact-graph
tap panel map.view
expect editor.map.mode is around
tap panel map.view
expect editor.map.view is all
expect editor.map.mode is all
tap panel map.pages
waitbg
expect editor.pages.grid is 3 3
expect editor.ticked is 0
shot 02-nine-previews
tap panel pages.page.2
expect editor.ticked is 1
expect editor.modal is pages
expect editor.page is 1
shot 03-tile-selected
# A second tap on the same tile opens it; no checkbox hit region.
tap panel pages.page.2
expect editor.modal is none
expect editor.page is 2
expect editor.screen_composed is 1
# A held tile offers a link from that page, with no ink added by the selection.
swipe 2 930 900 0 400
tap panel map.pages
touch down 318 550
wait 700
expect editor.pages.context is 1
touch up 318 550
expect editor.pages.context is 1
shot 04-held-page
tap panel pages.link
expect editor.modal is none
expect editor.link_mode is 1
expect editor.page is 1
pen line 1000 1100 1400 1300 20 0.6 0.6
pen leave
tap panel linkto.elsewhere
tap panel elsewhere.folder.Void/Nested
tap panel elsewhere.book.B
waitbg 5000
wait 200
expect bg.busy is 0
tap panel elsewhere.page.2
expect editor.destination is 0
expect editor.link_mode is 0
expect editor.strokes is 0
expect editor.screen_composed is 1
shot 05-picker-cleared
wait 4200
swipe 2 930 1400 0 -400
expect editor.bar is open
tap panel bar.areas
expect editor.eye_drawn is 3
expect editor.screen_composed is 1
shot 06-immediate-areas
tap panel bar.chip.1
expect editor.bar.peek is 2
expect editor.page is 1
expect editor.eye_drawn is 1
tap panel bar.areas
expect editor.eye_drawn is 0
tap panel bar.areas
expect editor.eye_drawn is 1
expect editor.bar.peek is 2
expect editor.page is 1
expect editor.screen_composed is 1
shot 06b-preview-areas
tap panel bar.close
expect editor.screen_composed is 1
# A pending pen hold must not replay against the page after leaving the dialog.
side on
tap panel settings
wait 900
side off
tap library done
wait 900
expect editor.eye_drawn is 3
expect editor.modal is none
# Settings remains hosted by the notebook through rotation and nested sheets.
tap panel settings
expect screen is editor
expect editor.modal is settings
expect status.front is 0
pen line 100 50 100 100 5 0.6 0.6
pen leave
expect editor.strokes is 0
expect editor.screen_composed is 1
shot 07-settings-over-notebook
tap library gestures
expect editor.settings.sheet is gestures
expect editor.screen_composed is 1
tap library done
expect editor.settings.sheet is settings
tap library controller
expect editor.settings.sheet is shortcuts
tap library done
expect editor.settings.sheet is settings
tap library display
expect editor.settings.sheet is display
expect status.shown is 1
expect editor.screen_composed is 1
key 45 0
expect editor.settings.sheet is settings
tap library pen-feel
expect screen is penset
tap 125 95
expect screen is editor
expect editor.settings.sheet is settings
expect editor.screen_composed is 1
rotate 1
wait 300
expect editor.modal is settings
expect editor.screen_composed is 1
tap library done
wait 900
expect editor.eye_drawn is 3
expect editor.modal is none
expect editor.notebook_id is A
expect editor.page is 1
expect editor.screen_composed is 1
swipe 2 930 900 0 400
tap panel map.pages
waitbg
expect editor.pages.grid is 3 3
shot 08-landscape-nine-previews
quit
