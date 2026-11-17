# settings: status_mode=1
# settings: theme=classic
# settings: font=literata
# env: GRIMOIRE_IMPORT_STABLE=fast GRIMOIRE_PDF_SLICE_PAGES=2
# copy: tests/fixtures/pdf/weblinks_wide import
# pdfs: 0
# rust-only
# A web link's card shows its whole address and its real host in the look whose letters are widest (Classic in
# Literata; 2.2, issue #174). The fixture's first link has the longest address the rules keep, 200 bytes of wide
# letters whose real host is at its end: the card draws it in Terminess, 5 lines, all of it, and its title, too long
# for the card, shows the host's end. The second link's address has a `\` (a browser reads it as `/`, so the host
# would not be the one its text names): refused, no link.
waitbg
wait 300
expect library.items is 1
tap 326 608
wait 700
expect screen is editor
expect font is literata
expect editor.weblinks is 1 1
expect editor.weblink.1 is 160 400 1040 160 https://www.example.org.WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW.WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW.WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW.evil.example/
tap 680 480
wait 400
shot 01-wide
expect editor.link_card is 1
expect editor.link_card.web is https://www.example.org.WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW.WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW.WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW.evil.example/
expect editor.link_card.title is www.example.org.wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww.wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww.wwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwwww.evil.example
expect editor.link_card.web_lines is 5 5
expect editor.link_card.web_title has .evil.example
expect editor.link_card.qr is 1
tap panel card.close
wait 300
expect editor.link_card is 0
# the address with a `\` is no link: a tap on it does nothing
tap 680 880
wait 300
expect editor.link_card is 0
pentap panel library
wait 600
expect screen is library
quit
