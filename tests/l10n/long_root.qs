# make l10n-check runs this in every language with GRIMOIRE_CHECK_FIT=1, from a data root whose
# path is longer than the Settings sheet is wide. The sheet's Data line holds that path, which is
# cut short as names are: the fit check must not count it, or make check would fail wherever the
# build directory is deep. (Upright, then sideways; the sheet then Done.)
wait 100
expect screen is library
tap 1535 95
wait 50
expect library.sheet is settings
rotate 1
wait 200
expect rotation is 1
expect library.sheet is settings
tap 1908 1695
wait 100
expect library.sheet is none
rotate 0
wait 200
expect rotation is 0
quit
