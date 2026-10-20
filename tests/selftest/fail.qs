# The runner's own check, the other way round: every expect line below fails, so the run must
# report each one and exit with status 3 (see the scenarios rule in the Makefile).
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
# lines that cannot be read
expect screen
expect screen was library
# Library > Exit: the app quits here, so the check after it never runs (that fails too)
tap 1725 95
expect screen is library
