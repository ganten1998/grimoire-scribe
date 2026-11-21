# settings: status_mode=1
# A PDF in grimoire/templates the worker refuses (2.2, #172): one that is no PDF and one with a password. Each is marked
# as a picture that does not read is: New notebook > More layouts... offers no template, says how to add some, and
# counts the two files it cannot use. Nothing of them becomes a notebook or a page.
# copy: tests/fixtures/pdf/junk templates
# copy: tests/fixtures/pdf/locked templates
# rust-only
wait 300
tap library add
tap library add.new-notebook
wait 200
tap library paper.more
wait 200
expect library.sheet is templates
waitbg
wait 300
# (marked while the sheet was open: the folder read again, and with no template to offer the sheet says how to add
# some and counts the two)
expect templates.count is 0
expect templates.names is Broken|Locked
shot 01-marked-at-once
tap library templates.back
wait 200
expect library.sheet is new
expect library.paper is 5
tap library paper.more
wait 200
expect library.sheet is templates
# (no template to offer: the sheet says how to add some and counts the two; their tiles, kept crossed out, are not
# drawn)
expect templates.count is 0
expect templates.names is Broken|Locked
shot 02-two-files-it-cannot-use
quit
