# settings: status_mode=1
# settings: files_card=1
# settings: shortcuts_enabled=1
# settings: home=files/documents/Sorted
# storage: 1
# generate: tests/files_home_tree.py
# copy: tests/fixtures/home-keys grimoire
# The path bar at a home inside Files (2.2, issue #160) reads the same however home was reached: at the start, and
# after the home action (H) from All storage, "Files > Documents > Sorted" (the nearest place that holds it), not
# the All storage way in.
# rust-only
wait 300
expect files.place is documents/Sorted
expect files.crumbs is Files > Documents > Sorted
tap library files.crumb.1
wait 300
tap library files.item.All_storage
wait 300
expect files.place is
tap library files.item.documents
wait 300
expect files.crumbs is Files > All storage > documents
key 35 0
wait 300
expect files.place is documents/Sorted
expect files.at_home is 1
expect files.crumbs is Files > Documents > Sorted
quit
