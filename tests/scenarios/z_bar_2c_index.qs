# rust-only
# copy: tests/fixtures/beta5-links .
# settings: mirror_pdf=0
# Legacy Index links remain usable through the page-status menu after the destination chooser retired them.
wait 300
tap 200 400
wait 1700
expect editor.link.1 is index
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect editor.bar.2 is index
shot 01-legacy-index-link
tap panel bar.chip.2
wait 300
expect editor.bar is 0
expect editor.modal is map
expect editor.map.list is 0
