# rust-only
# generate: tests/add_pages_fixture.py
# env: GRIMOIRE_SIM_CLOCK=script
# settings: shortcuts_enabled=1
# copy: tests/fixtures/new-pages-key .
# The count pad's - and + held (#119) count again as delete held deletes on every keyboard: one step at the
# touch-down, one more at 400 ms (KB_HOLD_MS) and one every 65 ms (KB_REPEAT_MS) after, until the lift or the
# pad's limit (1 for -, for + the most the notebook still holds), never more steps than the hold's time makes. By
# the script's clock (GRIMOIRE_SIM_CLOCK=script), so a busy machine's late waits change no count: each hold is
# waits of 100 ms, as the clock reaches the end of each wait with a tick there, and 100 ms is less than the three
# repeats a late tick makes up at once (KB_REPEAT_BATCH; one long wait's ticks can come bunched at its end on a
# busy machine, its time spent drawing not on the script's clock). The toolbar's pad, opened by the new-pages
# key (n): its + at 1374 2219, its - at 486 2219.
wait 300
tap library folder.Void
tap library notebook.S
wait 300
key 49 0
wait 200
expect editor.addpad is 1
expect editor.text is 1
# + held 1000 ms from the opening 1: the touch-down's step, then one at 400, 465 ... 985 ms: 11 steps, 12
touch down 1374 2219
wait 100
wait 100
wait 100
wait 100
wait 100
wait 100
wait 100
wait 100
wait 100
wait 100
expect editor.text is 12
touch up 1374 2219
wait 300
expect editor.text is 12
expect editor.addpad.done is Add 12 pages
# A tap: one step
tap panel addpad.key.inc
wait 100
expect editor.text is 13
# - held 500 ms: the touch-down's step, then 400 and 465 ms: 10
touch down 486 2219
wait 100
wait 100
wait 100
wait 100
wait 100
touch up 486 2219
wait 300
expect editor.text is 10
# - held 1 s, 11 steps by the clock: down to 1, no further
touch down 486 2219
wait 100
wait 100
wait 100
wait 100
wait 100
wait 100
wait 100
wait 100
wait 100
wait 100
touch up 486 2219
wait 300
expect editor.text is 1
expect editor.addpad.done is Add 1 page
tap panel addpad.key.cancel
wait 200
expect editor.addpad is 0
expect editor.page_ids is 1,2
# The notebook with room for five: + held 600 ms, 5 steps by the clock from the opening 1, stops at 5, with no
# toast
tap panel library
waitbg
tap library notebook.F
wait 300
key 49 0
wait 200
expect editor.addpad is 1
touch down 1374 2219
wait 100
wait 100
wait 100
wait 100
wait 100
wait 100
touch up 1374 2219
wait 300
expect editor.text is 5
expect editor.addpad.done is Add 5 pages
expect toast is
tap panel addpad.key.cancel
wait 200
expect editor.addpad is 0
expect editor.pages is 1995
quit
