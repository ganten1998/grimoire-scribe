# settings: status_mode=1
# settings: files_card=1
# settings: home=files/documents/../documents/Sorted
# storage: 1
# generate: tests/files_home_tree.py
# A settings.ini home= that names no home (2.2, issue #160): a ".." part is refused as the line is read, even where a
# path joined from it would reach a folder that is there (documents/Sorted). The Library's top, the one toast, and the
# line is gone from settings.ini.
# rust-only
wait 300
expect toast is Home folder not found.
expect settings.home is
expect files.open is 0
expect library.folder is
quit
