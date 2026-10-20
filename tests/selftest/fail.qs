# The runner's own check, the other way round: every line below that is not a comment, a wait or
# a tap fails, so the run must report each of them by its line number and exit with status 3 (see
# the scenarios rule in the Makefile).
wait 200
# a value that differs, a missing substring, a "not" that matches
expect screen is editor
expect screen has edit
expect screen not library
# the empty value is a value too: the first-run calibration tip is showing
expect toast is
# keys nobody answers
expect nonsense is 1
expect editor.nonsense is 1
expect library.nonsense is 1
expect settings.pencil.nonsense is 1
expect settings.nonsense.size_max_mm is 1
# lines that cannot be read
expect screen
expect screen was library
# misspelt checks and other lines the runner does not know: none is skipped without a word
expcet screen is library
Expect screen is library
expects screen is library
tpa 965 95
storage sideways
# the app is running, and nothing is watched (the share server is not running)
expect running is 0
expect watching is 1
# Library > Exit: the app quits here, so a check after it cannot see the app (that fails too),
# and "running" is 0 from now on
tap 1725 95
expect screen is library
expect watching is 0
expect running is 1
expect running not 0
