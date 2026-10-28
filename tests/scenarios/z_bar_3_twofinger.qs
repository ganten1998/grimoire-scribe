# settings: status_mode=1
# rust-only
# Two fingers down during a peek close the bar back to the page it began on, not Pages: the first finger of the
# gesture lands on the page above the bar, where a lift would stay, and the second finger's landing drops that.
# root: z_bar_0_make
wait 300
tap 200 400
wait 300
swipe 2 930 1300 0 -400
wait 300
tap 479 2218
wait 300
expect editor.bar is peek
expect editor.bar.peek is 3
swipe 2 930 900 0 400
wait 300
expect editor.bar is 0
expect editor.bar.peek is 0
expect editor.page is 1
expect editor.modal is none
expect touch.gesture is 2.down
expect notebook.last_page is 1
