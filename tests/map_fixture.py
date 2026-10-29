#!/usr/bin/env python3
"""Deterministic metadata-only map fixtures. Never touches a user's data root."""
import pathlib,struct,sys
count=int(sys.argv[1]);root=pathlib.Path(sys.argv[2]);book=root/'notebooks'/'map-fixture'
book.mkdir(parents=True,exist_ok=True)
(book/'notebook.ini').write_text('# Grimoire notebook\nname=Map fixture\nfolder=\npaper=4\ncreated=1\nmodified=1\nlast_page=2\nnext_page_id=%d\npage_w=1860\npage_h=2480\npages=%s\n'%(count+1,','.join(map(str,range(1,count+1)))))
blank=struct.pack('<4sHHIII',b'QPG1',1,4,1860,2480,0)
for i in range(1,count+1):(book/f'p{i}.qpg').write_bytes(blank)
names=[3,7,11,15,19,23] if count==40 else []
(book/'nav.ini').write_text('# Grimoire page names\n'+''.join(f'name.{i}=Section {i}\n' for i in names))
links=[]
if count==40:
 for i in range(3,26):links.append((i,1,3+(i-2)%23))
 for i in range(3,10):links.append((i,2,i+7))
elif count!=250:
 for i in range(1,count+1):
  for seed,step in [(1,1),(2,31)]:links.append((i,seed,1+(i-1+step)%count))
(book/'links.ini').write_text('# Grimoire links\n'+''.join(f'link.{a}={seed:08x},p{b}\narea.{a}={seed:08x},300,{500+seed*300},400,200\n' for a,seed,b in links))
(root/'shortcuts.ini').write_text('key.k=links\nkey.b=go-back\nkey.x=close-menu\n')
with (root/'settings.ini').open('a') as f:f.write('\nshortcuts_enabled=1\nmirror_pdf=0\nstatus_mode=1\n')
