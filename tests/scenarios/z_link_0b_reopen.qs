# The migrated areas were saved (z_link_0_migrate): opened again, the notebook has nothing left to
# migrate and nothing waiting to be written (page 4, never opened, was migrated by the idle ticks and
# saved: nothing migrates at this open), and its links keep their areas. (Page 1's link D, whose
# ring was undone in beta.5, has no ink and so no area: it is tried at the page's load and stays.)
# root: z_link_0_migrate
# rust-only
wait 300
tap 200 400
wait 200
expect screen is editor
expect editor.page is 1
expect editor.links_dirty is 0
expect editor.migrate_left is 0
expect editor.links is 4 3
expect editor.link.2 is p3
expect editor.link.2.area is 300 1000 600 200
expect editor.link.3.area is
tap 600 1100
expect editor.page is 3
pentap panel library
waitbg
expect screen is library
