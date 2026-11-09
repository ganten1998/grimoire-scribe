# settings: status_mode=1
# storage: 1
# copy: tests/fixtures/files .
# generate: tests/files_hostile.py
# The picker over a hostile tree: links to /etc, to "..", to a PDF, to a folder of pictures and to Grimoire's
# notebooks, a FIFO named .pdf, a name with ':' and one that is not UTF-8. Links, the FIFO and the odd name are
# not listed; nothing is entered or opened through a link; the plain files and folders are. A mirror PDF and a
# trashed file are not offered anywhere, and a job that names one (here the runner's pdfimport) is refused.
# rust-only
wait 300
tap library new
wait 200
tap library new.file
wait 300
tap library files.item.documents
wait 300
shot 01-documents
expect files.rows is 4
expect files.item.Books is Books
expect files.item.Scans is Scans
expect files.item.Planner.pdf is Planner.pdf
expect files.item.time_10:30.pdf is time 10:30.pdf
expect files.item.Alias.pdf is
expect files.item.Hosts.pdf is
expect files.item.pipe.pdf is
expect files.item.etc is
expect files.item.up is
expect files.item.LinkedScans is
expect files.hidden is 5
tap library files.item.Scans
wait 400
expect files.place is documents/Scans
expect files.rows is 0
tap library files.back
wait 300
expect files.place is documents
tap library files.back
wait 300
expect files.place is root
tap library files.item.Downloads
wait 300
expect files.rows is 1
expect files.item.Daily.pdf is Daily.pdf
expect files.item.nb is
tap library files.back
wait 200
tap library files.row.4
wait 300
shot 02-all
expect files.place is
expect files.item.documents is documents
expect files.item.grimoire is grimoire
expect files.item.system is
tap library files.item.grimoire
wait 300
expect files.place is grimoire
expect files.item.import is import
expect files.item.PDFs is
expect files.item.trash is
expect files.item.notebooks is
tap library files.back
wait 200
tap library files.back
wait 200
tap library files.back
wait 200
expect files.open is 0
expect library.sheet is new
tap library done
wait 100
pdfimport grimoire/PDFs/School/Mirror.pdf
pdfimport grimoire/trash/Trashed.pdf
pdfimport grimoire/pdfs/School/Mirror.pdf
pdfimport grimoire/notebooks/x.pdf
pdfimport documents/Alias.pdf
waitbg
wait 300
expect library.items is 0
quit
