# root: z_annotate_g_paused
# env: GRIMOIRE_PASSCODE=paused
# The start after z_annotate_g_paused: the passcode's notice, which that start's toast went without,
# is told now.
wait 300
expect annotate.result is none
expect toast is Your Kindle passcode is paused while Grimoire is open.
quit
