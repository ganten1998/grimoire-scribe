# settings: status_mode=1
# settings: files_card=1
# settings: home=files/documents/Linked
# storage: 1
# generate: tests/files_home_tree.py
# A home that names a link (2.2, issue #160): documents/Linked leads to Sorted, but Files' walk follows no link, so
# it is a home refused, as one gone is: the Library's top, the one toast, and home is Notebooks again. Files was not
# opened there, and the link is as it was.
# rust-only
wait 300
expect toast is Home folder not found.
expect settings.home is
expect files.open is 0
expect library.folder is
expect storage.exists.documents/Linked is 1
quit
