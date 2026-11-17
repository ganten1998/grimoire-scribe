# settings: status_mode=1
# root: z_weblinks_1_card
# env: GRIMOIRE_LANG=es
# env: GRIMOIRE_CHECK_FIT=1
# rust-only
# A web link's card in Spanish, every text checked for fit (2.2, issue #174): the line over the code and Done are
# the catalog's own strings (the About page's line, the sheets' Done), the address and its host are shown as
# stored. Page 2's address takes two lines. Upright, then sideways.
wait 300
tap 326 608
wait 700
expect screen is editor
expect editor.page is 2
tap 480 960
wait 400
shot 01-card-es
expect editor.link_card is 1
expect editor.link_card.rows is close
expect editor.link_card.web is https://example.org/a/long/address/that/goes/on/two/lines
expect editor.link_card.qr is 1
tap panel card.close
wait 300
expect editor.link_card is 0
# sideways the page is fitted at 3/4 and centred: the second line's link is at 662..1142, 660..780
rotate 1
wait 600
tap 900 720
wait 400
shot 02-card-sideways
expect editor.link_card is 1
expect editor.link_card.web is https://example.org/a/long/address/that/goes/on/two/lines
expect editor.link_card.qr is 1
tap panel card.close
wait 300
expect editor.link_card is 0
rotate 0
wait 600
pentap panel library
wait 600
expect screen is library
quit
