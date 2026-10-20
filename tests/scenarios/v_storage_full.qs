# The store is full (the simulator's "storage full": every file write fails). The autosave says so
# and tries again every 2 s, and holding the power button 4 s never claims a save it could not
# make: awake, the first hold stays and says why; once there is room the page saves; and full
# again, a second hold within the minute leaves without the page.
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
pen line 300 900 1300 950 40 0.6 0.6
wait 2600
expect editor.unsaved is 0
storage full
pen line 300 1200 1300 1250 40 0.6 0.6
wait 2600
expect editor.unsaved is 1
expect toast has Could not save this page
power down
power held
power up
expect running is 1
expect toast is Could not save this page. Hold power 4 s again to leave anyway.
shot 01-not-saved-staying
storage ok
wait 2600
expect editor.unsaved is 0
expect editor.strokes is 2
storage full
pen line 300 1500 1300 1550 40 0.6 0.6
wait 2600
expect editor.unsaved is 1
power down
power held
wait 300
expect running is 0
