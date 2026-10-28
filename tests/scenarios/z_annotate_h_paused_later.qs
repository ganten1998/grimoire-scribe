# settings: status_mode=1
# root: z_annotate_g_paused
# env: GRIMOIRE_PASSCODE=enabled
# After the annotation failure, no obsolete "PIN paused" notice appears.
wait 300
expect annotate.result is none
expect native_pin is 1
expect toast not passcode is paused
quit
