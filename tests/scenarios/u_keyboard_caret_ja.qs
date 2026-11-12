# The caret on the kana pad, in Japanese (2.1.0-beta.2, issue #36), with every string's fit checked: the arrows
# are on the kana pad's text line as on every page, a kana typed goes in at the caret, ゛小 changes the kana before
# the caret, the edit page's way back is あ (to the kana pad), and the edit page's words fit (still English: the
# translator's to give).
# rust-only
# env: GRIMOIRE_LANG=ja GRIMOIRE_CHECK_FIT=1
clock 1700000000
wait 3600
expect language is ja
tap 1155 95
wait 100
expect library.sheet is keyboard
# the kana key (the accents key's place), then か, た
tap 747 2365
wait 100
tap 930 1725
tap 340 1885
expect library.caret is かた|
# back over た, then あ typed there, then ゛小 on it (あ to ぁ)
tap 1549 1570
expect library.caret is か|た
tap 340 1725
expect library.caret is かあ|た
tap 930 2205
expect library.caret is かぁ|た
wait 1200
shot 01-kana-caret
# the edit page: its way back reads あ and goes to the kana pad
tap 1372 1570
wait 100
shot 02-edit-page
tap 930 1751
expect library.caret is [かぁた|]
tap 930 1964
expect keyboard.clipboard is かぁた
tap 1372 2177
expect library.caret is かぁた|
tap 393 2365
wait 100
tap 1520 1725
expect library.caret is かぁたさ|
tap 175 2365
wait 100
expect library.sheet is none
# sideways
rotate 1
wait 300
tap library new-folder
wait 100
tap 1837 950
wait 100
shot 03-edit-sideways
tap 221 1745
wait 100
expect library.sheet is none
quit
