# settings: status_mode=1
# settings: files_card=1
# settings: files_view=covers
# storage: 1
# generate: tests/files_place_tree.py
# copy: tests/fixtures/pdf/planner documents/Covers
# copy: tests/fixtures/pdf/junk documents/Covers
# copy: tests/fixtures/photos documents/Covers
# copy: tests/fixtures/fonts/good documents/Covers
# copy: tests/fixtures/pictures documents/Covers
# copy: tests/fixtures/files/documents/Scans documents/Covers
# Files' Covers view (2.2, issue #145): a preview that leaves the screen and comes back is read from the cache again,
# never left as its kind's icon. documents/Covers holds 24 files Grimoire can use, three screens of nine tiles: PDFs,
# photos, fonts and pictures (three of them ones the worker refuses, whose tiles are their icons from the start), and
# a text file that only Show all lists. Each way back to a screen whose previews were made: Prev after Next; Show
# (which lists the folder again from its first screen), both ways; Sort, by size (other files first) and round to
# the names again; the Kindle turned sideways (a tenth tile) and upright, twice. Coming back makes nothing again:
# no more files are asked for than the 24, and no job runs.
# rust-only
wait 300
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Covers
wait 300
expect files.view is covers
expect files.rows is 24
expect files.per_page is 9
waitfor files.tiles is pppiipppp
expect files.asked is 9
shot 01-first
tap library files.next
waitfor files.tiles is ppppppppp
tap library files.next
waitfor files.tiles is pppppp
expect files.asked is 24
# Prev, twice: each screen is its previews again
tap library files.prev
waitfor files.tiles is ppppppppp
tap library files.prev
waitfor files.tiles is pppiipppp
shot 02-prev
# Show all from the second screen, and back to the usable files: the first screen each time
tap library files.next
waitfor files.tiles is ppppppppp
tap library files.show
waitfor files.rows is 25
waitfor files.tiles is pppiipppp
tap library files.next
waitfor files.tiles is ppppppppi
tap library files.show
waitfor files.rows is 24
waitfor files.tiles is pppiipppp
# Sort: newest (the files' times tie, as the generator sets them: the same screen), size (other files come first),
# then names again
tap library files.sort
wait 300
expect files.sort is newest
waitfor files.tiles is pppiipppp
tap library files.sort
waitfor files.tiles is ppppppppi
expect files.sort is size
tap library files.sort
waitfor files.tiles is pppiipppp
expect files.sort is name
shot 03-sorted-back
# sideways: a tenth tile comes; upright: it goes; sideways again: it is its preview again
rotate 1
waitfor files.tiles is pppiippppp
expect files.per_page is 10
rotate 0
waitfor files.tiles is pppiipppp
rotate 1
waitfor files.tiles is pppiippppp
shot 04-sideways-again
rotate 0
waitfor files.tiles is pppiipppp
expect files.making is 0
expect files.asked is 24
expect bg.busy is 0
quit
