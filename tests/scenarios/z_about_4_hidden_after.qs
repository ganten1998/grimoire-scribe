# root: z_about_3_hidden
# The start after z_about_3_hidden's: the notes were seen, so What's new does not open again, and
# settings.ini still has whatsnew_hide=1 as it was written back.
# rust-only
wait 300
expect screen is library
expect library.sheet is none
expect settings.whatsnew_hide is 1
expect settings.whatsnew_seen not 1.2.0
quit
