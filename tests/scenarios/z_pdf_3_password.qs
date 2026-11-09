# settings: status_mode=1
# env: GRIMOIRE_IMPORT_STABLE=fast
# copy: tests/fixtures/pdf/locked import
# pdfs: 0
# A PDF that needs a password is refused before any page is drawn: a toast says so, no notebook is made, and the
# file stays in import/ (it is tried again only when it changes).
# rust-only
waitbg
wait 300
shot 01-refused
expect toast is PDF needs a password.
expect library.items is 0
quit
