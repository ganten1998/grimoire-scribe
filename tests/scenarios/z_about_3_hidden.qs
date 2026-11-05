# settings: settings_version=3
# settings: whatsnew_seen=1.2.0
# settings: whatsnew_hide=1
# The owner's rule of 2026-11-05: every update opens What's new once, and there is no Don't show this
# again. A settings.ini that has the 2.0.0 betas' box ticked (whatsnew_hide=1, as the owner's Scribe
# has it) and an older version's notes seen still opens the notes at the first start after the
# update. Dismiss closes them with this version's recorded as seen, and whatsnew_hide stays as it was
# for an older Grimoire that reads the file; z_about_4_hidden_after: the next start opens the Library
# alone.
# rust-only
wait 300
expect screen is library
expect library.sheet is about
expect library.about.full is 0
expect settings.whatsnew_hide is 1
shot 01-after-update
tap library about.close
wait 100
expect library.sheet is none
expect settings.whatsnew_seen not 1.2.0
expect settings.whatsnew_seen not
expect settings.whatsnew_hide is 1
quit
