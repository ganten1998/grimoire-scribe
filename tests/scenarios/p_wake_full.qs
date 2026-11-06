# rust-only
# After a wake the screen is pushed again 2.5 s later as a FULL refresh of the whole screen (app_wake,
# WAKE_REDRAW_MS). The owner's Scribe woke with parts of the KOReader wallpaper still on the panel while
# the framebuffer held the Library whole (2026-11-05): the Kindle had drawn on the panel after the wake's
# first refresh, and the second push was a no-flash one, which drives only the pixels the controller
# thinks changed. A full refresh drives every pixel. Not before 2.5 s: the Kindle's drawing comes first.
# settings: sleep_mode=3
wait 200
expect screen is library
sleep
wait 400
expect asleep is 1
wake
wait 2200
expect asleep is 0
expect display.presents not -1
wait 600
expect display.presents is full 0 0 1860 2480
quit
