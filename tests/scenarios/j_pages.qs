# Pages view: previews, tick pages 2 and 4, delete; open a page from its preview; select all + delete
wait 200
tap 965 95
wait 100
tap 765 656
tap 1564 1934
wait 300
pen curve 300 500 40 60 0.7 1
pentap 1768 1438
wait 200
pen curve 300 500 40 60 0.7 2
pentap 1768 1438
wait 200
pen curve 300 500 40 60 0.7 3
pentap 1768 1438
wait 200
pen curve 300 500 40 60 0.7 4
pentap 1768 1438
wait 200
pen curve 300 500 40 60 0.7 5
wait 200
pentap 1768 1350
wait 400
shot 01-pages-view
tap 1150 296
tap 547 1173
wait 200
shot 02-ticked-2-and-4
tap 1630 95
wait 400
shot 03-after-delete
tap 326 608
wait 400
shot 04-opened-page-1
pentap 1768 1350
wait 400
tap 1250 95
wait 200
tap 1630 95
wait 400
shot 05-all-deleted
tap 125 95
wait 300
shot 06-back-in-editor
quit
