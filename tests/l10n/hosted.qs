# make l10n-check runs this in every language, Gothic and Classic, with GRIMOIRE_CHECK_FIT=1 and
# GRIMOIRE_HOST=koreader: the Library's top bar as KOReader's Grimoire plugin opens it, Back to
# KOReader in Exit's place, a button as wide as its words (upright, then sideways).
wait 100
expect host is koreader
expect screen is library
shot 01-library-hosted
rotate 1
wait 200
expect rotation is 1
shot 02-library-hosted-sideways
rotate 0
wait 200
expect rotation is 0
quit
