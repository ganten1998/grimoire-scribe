# settings: status_mode=1
# A template PDF past the 50 pages a template PDF may have (2.2, #172): a planner of 51 pages is no template as a
# whole, never its first 50: its one tile is crossed out; beside it a PDF of two pages gives its two tiles. New
# notebook > More layouts... shows the two after the crossed-out one, and a page picked is the new notebook's paper.
# copy: tests/fixtures/pdf/tpl-cap templates
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
waitbg
wait 300
expect templates.count is 2
expect templates.ready is 2
expect templates.names is Long Planner|Weekly p. 1|Weekly p. 2
shot 01-the-long-one-crossed-out
# the second tile, Weekly p. 1
tap 511 452
wait 200
expect library.sheet is new
expect library.paper is template
quit
