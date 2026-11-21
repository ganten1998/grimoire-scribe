# settings: status_mode=1
# rust-only
# The notebook opens again on the page the peek began on: notebook.ini kept the origin.
# root: z_bar_6_leave
wait 300
tap library card.1
wait 400
expect screen is editor
expect editor.page is 1
expect editor.bar is 0
