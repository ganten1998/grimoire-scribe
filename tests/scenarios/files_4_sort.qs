# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_place_tree.py
# Files' Sort and Show (2.2): documents/Sorted holds a folder and files of known sizes and times. Sort by name, by
# the newest, by the largest (the folder first each time: every row is stat-ed, then the rows are sorted at once);
# Show all adds the file Grimoire cannot use and a link, greyed, "link, not followed". What was chosen stays for the
# next time Files opens (files_sort, files_show).
# rust-only
wait 300
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Sorted
wait 300
expect files.sort is name
expect files.show is usable
expect files.order is Sub|a.pdf|b.pdf|c.png|d.ttf
expect files.kind.2 is pdf
expect files.kind.4 is picture
expect files.kind.5 is font
tap library files.sort
waitfor files.sorting is 0
expect files.sort is newest
expect files.order is Sub|c.png|b.pdf|d.ttf|a.pdf
expect files.drawn is Sub|c.png|b.pdf|d.ttf|a.pdf
shot 01-newest
tap library files.sort
waitfor files.sorting is 0
expect files.sort is size
expect files.order is Sub|b.pdf|c.png|a.pdf|d.ttf
expect files.drawn is Sub|b.pdf|c.png|a.pdf|d.ttf
tap library files.show
waitfor files.sorting is 0
expect files.show is all
expect files.order is Sub|e.txt|b.pdf|c.png|a.pdf|d.ttf|f.pdf
expect files.drawn is Sub|e.txt|b.pdf|c.png|a.pdf|d.ttf|f.pdf
expect files.kind.2 is other
expect files.kind.7 is link
expect files.sub.7 is link, not followed
shot 02-size-all
tap library files.sort
waitfor files.sorting is 0
expect files.sort is name
expect files.order is Sub|a.pdf|b.pdf|c.png|d.ttf|e.txt|f.pdf
expect files.drawn is Sub|a.pdf|b.pdf|c.png|d.ttf|e.txt|f.pdf
tap library files.item.f.pdf
wait 200
expect files.card is
expect files.place is documents/Sorted
tap library files.crumb.1
wait 300
tap library files.back
wait 300
expect files.open is 0
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Sorted
wait 300
expect files.sort is name
expect files.show is all
expect files.rows is 7
quit
