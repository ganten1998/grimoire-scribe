# settings: status_mode=1
# settings: files_card=1
# settings: shortcuts_enabled=1
# settings: home=files/documents/Sorted
# storage: 1
# generate: tests/files_home_swap_tree.py
# copy: tests/fixtures/home-keys grimoire
# The home folder swapped for a link while Files shows it (2.2, issue #160): over USB documents/Sorted is moved
# away and a link out of the storage (to /) put in its place. Esc no longer asks "Exit Grimoire?": the home folder
# does not open as a home (the walk follows no link), so Esc goes up as from any folder that is gone, to documents,
# where the link is not listed. The home action (H) then finds home refused: the Library's top, "Home folder not
# found.", and home is Notebooks again. The link is never followed.
# rust-only
wait 300
expect files.place is documents/Sorted
expect files.at_home is 1
storage move documents/Sorted documents/Was
storage move documents/Root documents/Sorted
expect storage.exists.documents/Sorted is 1
key 1 0
wait 300
expect library.sheet is none
expect files.open is 1
expect files.place is documents
expect files.at_home is 0
expect files.order is Was
key 35 0
wait 300
expect files.open is 0
expect library.folder is
expect toast is Home folder not found.
expect settings.home is
quit
