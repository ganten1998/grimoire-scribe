# settings: status_mode=1
# A background job that changes no card does not draw the Library again: here the templates'
# previews, made after New notebook's More papers... was opened and left. (One that changes a
# card does: z_tpl_8.) The job is slowed so that it ends with the Library in front.
# copy: tests/fixtures/scen-tpl-restore .
# copy: tests/fixtures/scen-templates templates
# env: GRIMOIRE_BG_DELAY_MS=3000
wait 500
expect screen is library
expect library.items is 1
expect library.card.1 is thumb
tap 965 95
wait 100
expect library.sheet is new
tap 300 1650
wait 200
expect library.sheet is templates
expect templates.ready is 0
tap 300 2123
wait 100
expect library.sheet is new
tap 300 2250
wait 100
expect library.sheet is none
expect bg.busy is 1
# (the previews: a slowed job, waited for)
waitbg
wait 300
expect bg.busy is 0
expect library.cards_redrawn is 0
expect library.card.1 is thumb
expect library.card.1.luma is 255
quit
