# settings: status_mode=1
# A PDF in grimoire/templates the worker refuses (2.2, #172): one that is no PDF and one with a password. Each is marked
# as a picture that does not read is: New notebook > More layouts... offers no template, says how to add some, and
# counts the two files it cannot use. Nothing of them becomes a notebook or a page.
# copy: tests/fixtures/pdf/junk templates
# copy: tests/fixtures/pdf/locked templates
# rust-only
wait 300
tap library new
wait 200
tap library paper.more
wait 200
expect library.sheet is templates
waitbg
wait 300
# (marked while the sheet was open: crossed out in their places, offered no more, until it opens again)
expect templates.count is 0
expect templates.names is Broken|Locked
shot 01-crossed-out
tap library templates.back
wait 200
expect library.sheet is new
expect library.paper is 5
tap library paper.more
wait 200
expect library.sheet is templates
expect templates.count is 0
expect templates.names is
shot 02-two-files-it-cannot-use
quit
