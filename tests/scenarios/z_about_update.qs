# settings: settings_version=3
# settings: whatsnew_seen=1.2.0
# What's new opens by itself at the first start after an update (a settings file a Grimoire saved,
# its notes seen another version's): once, this version's then recorded (z_about_update_2).
# rust-only
wait 300
expect screen is library
expect library.sheet is about
expect library.about.full is 0
shot 01-after-update
tap library about.close
wait 100
expect library.sheet is none
quit
