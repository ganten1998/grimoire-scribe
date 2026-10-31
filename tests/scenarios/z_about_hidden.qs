# settings: settings_version=3
# settings: whatsnew_seen=1.2.0
# settings: whatsnew_hide=1
# Don't show this again ticked: the first start after an update opens the Library alone.
# rust-only
wait 300
expect screen is library
expect library.sheet is none
quit
