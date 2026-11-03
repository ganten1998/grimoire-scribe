#!/usr/bin/env python3
"""Deterministic metadata-only map fixtures. Never touches a user's data root.

tests/map_fixture.py COUNT ROOT: one notebook of COUNT pages (map-fixture).
tests/map_fixture.py --library BOOKSxPAGES ROOT: a library for the Graph of all notebooks (2.0.0-beta.2):
BOOKS notebooks lib000, lib001, ... (folder Library, "Book N"), PAGES pages each (ids 1..PAGES, as every
notebook starts at 1), a name on every tenth page. In each: page 1 links to its page 2 and to page 2 of the
next notebook (the last to the first's), page 3 links to the Index; with more than 20 pages, about 1.1 links
a page more from a seeded generator, one in twenty to another notebook."""
import pathlib,struct,sys
blank=struct.pack('<4sHHIII',b'QPG1',1,4,1860,2480,0)

def notebook(root,nid,name,folder,pages,names,links,last_page=0):
    book=root/'notebooks'/nid
    book.mkdir(parents=True,exist_ok=True)
    (book/'notebook.ini').write_text('# Grimoire notebook\nname=%s\nfolder=%s\npaper=4\ncreated=1\nmodified=1\nlast_page=%d\nnext_page_id=%d\npage_w=1860\npage_h=2480\npages=%s\n'%(name,folder,last_page,max(pages)+1,','.join(map(str,pages))))
    for i in pages:(book/f'p{i}.qpg').write_bytes(blank)
    (book/'nav.ini').write_text('# Grimoire page names\n'+''.join(f'name.{p}={t}\n' for p,t in names))
    (book/'links.ini').write_text('# Grimoire links\n'+''.join(f'link.{a}={seed:08x},{t}\narea.{a}={seed:08x},300,{500+(seed%5)*300},400,200\n' for a,seed,t in links))

def library(root,spec):
    books,pages=map(int,spec.split('x'))
    state=[20261102]
    def rnd(n):
        state[0]=(state[0]*6364136223846793005+1442695040888963407)%(1<<64)
        return (state[0]>>33)%n
    for k in range(books):
        nid=f'lib{k:03d}'
        nxt=f'lib{(k+1)%books:03d}'
        links=[(1,1,'p2'),(1,2,f'n:{nxt}:p2'),(3,3,'index')] if pages>=3 else []
        if pages>20:
            seen=set()
            for s in range(int(pages*1.1)):
                a=1+rnd(pages)
                if rnd(20)==0 and books>1:
                    o=rnd(books)
                    if o==k:o=(o+1)%books
                    t=f'n:lib{o:03d}:p{1+rnd(pages)}'
                else:
                    t=f'p{1+rnd(pages)}'
                if (a,t) in seen or t==f'p{a}':continue
                seen.add((a,t));links.append((a,10+s,t))
        notebook(root,nid,f'Book {k}','Library',list(range(1,pages+1)),[(p,f'Topic {k}.{p}') for p in range(1,pages+1,10)],links)

root=pathlib.Path(sys.argv[-1])
if sys.argv[1]=='--library':
    library(root,sys.argv[2])
else:
    count=int(sys.argv[1])
    names=[3,7,11,15,19,23] if count==40 else []
    links=[]
    if count==40:
        for i in range(3,26):links.append((i,1,3+(i-2)%23))
        for i in range(3,10):links.append((i,2,i+7))
    elif count!=250:
        for i in range(1,count+1):
            for seed,step in [(1,1),(2,31)]:links.append((i,seed,1+(i-1+step)%count))
    notebook(root,'map-fixture','Map fixture','',list(range(1,count+1)),[(i,f'Section {i}') for i in names],[(a,seed,f'p{b}') for a,seed,b in links],last_page=2)
(root/'shortcuts.ini').write_text('key.k=links\nkey.b=go-back\nkey.x=close-menu\nkey.m=index-map\n')
with (root/'settings.ini').open('a') as f:f.write('\nshortcuts_enabled=1\nmirror_pdf=0\nstatus_mode=1\n')
