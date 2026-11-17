# settings: status_mode=1
# settings: files_card=1
# settings: shortcuts_enabled=1
# storage: 1
# generate: tests/files_home_tree.py
# copy: tests/fixtures/home-keys grimoire
# A Library folder as home (2.2, issue #160): School's menu has Set as home, and once School is home the same row is
# Reset home to Notebooks (tapped, home is Notebooks again; tapped once more, School is home). Nothing is said. At
# School its Back button, the one way up there, stays, and Esc does what it does: up to the Library's top, where Esc
# asks "Exit Grimoire?" as always; in a folder inside it Esc goes up to it. Renamed with its menu's Rename folder,
# School takes home with it.
# files_26_home_library_start starts from here.
# rust-only
wait 300
expect settings.home is
tap library folder-menu.School
wait 300
expect library.sheet is folder-more
shot 01-folder-menu
tap library fmore.home
wait 300
expect library.sheet is none
expect toast is
expect settings.home is notebooks/School
tap library folder-menu.School
wait 300
shot 02-folder-menu-home
tap library fmore.home
wait 300
expect settings.home is
tap library folder-menu.School
wait 300
tap library fmore.home
wait 300
expect settings.home is notebooks/School
# at home: Esc goes up, as its Back button does; inside it Esc goes up to it, and Back leads up
tap library folder.School
wait 300
expect library.folder is School
expect library.ui_has.back is 1
key 1 0
wait 300
expect library.sheet is none
expect library.folder is
shot 03-esc-at-home
tap library folder.School
wait 300
tap library folder.School/2026
wait 300
expect library.folder is School/2026
key 1 0
wait 300
expect library.folder is School
expect library.sheet is none
tap library back
wait 300
expect library.folder is
# at the Library's top Esc asks "Exit Grimoire?", as always, and No stays
key 1 0
wait 300
expect library.sheet is exit
key 1 0
wait 300
expect library.sheet is none
expect library.folder is
# School renamed Uni: home goes with it
tap library folder-menu.School
wait 300
tap library fmore.rename
wait 300
expect library.text is School
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
expect library.text is
tap 175 2205
tap 1195 1885
tap 1278 2205
tap 1372 1885
expect library.text is Uni
tap 1643 2365
wait 300
expect library.folders is Uni, Uni/2026
expect settings.home is notebooks/Uni
quit
