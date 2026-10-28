# settings: status_mode=1
# The work pump (common.h): the app's long work calls it as it goes. On the Kindle it reads the pens
# meanwhile, so a page's render (1.2 s on a dense page), a save's fsyncs or a full-screen copy no
# longer lets the kernel drop their events; the simulator only counts the calls. pump.calls is how
# many came since it was last checked.
# rust-only
wait 300
tap 965 95
wait 200
tap 510 460
wait 100
tap 1564 2254
wait 400
expect screen is editor
pen curve 400 600 50 200 0.6 6
pen curve 900 1400 80 300 0.6 8
wait 300
expect editor.strokes is 2
# (the start and the strokes: read, so that the next check counts only what follows)
expect pump.calls not -1
# A new page: page 1 is saved (a pump after each step of each file written) and page 2 drawn.
pentap panel next
wait 400
expect editor.page is 2
expect pump.calls not 0
# Back: page 1's two strokes are drawn again, with a pump after each and every 128 points.
pentap panel previous
wait 400
expect editor.page is 1
expect editor.strokes is 2
expect pump.calls not 0
shot 01-back
quit
