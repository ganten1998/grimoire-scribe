# rust-only
# root: z_lasso_2_link_copy
# settings: mirror_pdf=0
# A linked item pasted into another notebook (z_lasso_2) still goes where it went: a finger's tap on its
# area there, its target rewritten n:<source>:p3, opens the first notebook at its page 3 (design 10).
wait 300
tap 322 620
wait 500
expect screen is editor
expect editor.pages is 1
expect editor.links is 1 1
expect editor.page_links has :p3@300,1200,600,200
tap 600 1300
wait 800
expect screen is editor
expect editor.pages is 5
expect editor.page is 3
quit
