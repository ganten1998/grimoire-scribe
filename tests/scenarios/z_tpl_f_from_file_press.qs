# settings: status_mode=1
# storage: 1
# copy: tests/fixtures/scen-templates documents/Pics
# copy: tests/fixtures/scen-templates grimoire/templates
# env: GRIMOIRE_BG_DELAY_MS=3000
# New notebook > More layouts... > From a file... (2.2, #146) with a finger down on the sheet when the copy ends. The
# folder is not read again under the finger (that would forget its press, and its lift would then close the sheet as
# a tap outside it does): the sheet stays open, and the new template's tile comes once the finger is up.
# rust-only
wait 300
tap library new
wait 200
tap library paper.more
wait 300
waitbg
wait 300
expect templates.names is Dots|Lined
expect templates.ready is 2
tap library templates.from_file
wait 300
tap library files.item.documents
wait 300
tap library files.item.Pics
wait 300
tap library files.item.Lined.png
wait 300
tap library files.card.template
wait 300
expect library.sheet is templates
expect templates.copying is 1
expect bg.children is 1
# a finger down on the sheet, on none of its buttons, until the copy has ended
touch down 930 1400
waitfor bg.children is 0
wait 300
expect templates.copying is 1
expect templates.names is Dots|Lined
touch up 930 1400
wait 300
expect library.sheet is templates
waitbg
wait 300
waitbg
wait 300
expect templates.copying is 0
expect templates.names is Dots|Lined 2|Lined
expect templates.ready is 3
expect toast is
shot 01-after-the-lift
quit
