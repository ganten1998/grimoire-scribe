# root: z_about_1_update
# The start after the update's first: What's new does not open again (the bat still opens it).
# rust-only
wait 300
expect screen is library
expect library.sheet is none
quit
