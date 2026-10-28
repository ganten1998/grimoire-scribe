# A template that cannot go into the notebook (here a file stands where the notebook keeps its
# pictures, bg/): picking it says so and changes nothing, the page stays on its paper with nothing
# to undo.
# copy: tests/fixtures/scen-tpl-nobg .
# copy: tests/fixtures/scen-templates templates
# rust-only
wait 500
expect screen is library
tap 326 608
wait 600
expect screen is editor
expect editor.paper is 1
pentap panel paper
wait 200
pentap 232 1750
waitbg
wait 300
expect editor.modal is templates
expect templates.ready is 2
pentap 511 452
wait 300
expect editor.modal is none
expect toast is Could not use that template (see grimoire.log).
expect editor.paper is 1
expect editor.unsaved is 0
shot 01-could-not
pentap 1768 594
wait 300
expect editor.paper is 1
quit
