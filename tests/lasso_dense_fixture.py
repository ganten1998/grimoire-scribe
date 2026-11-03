#!/usr/bin/env python3
"""A notebook with one dense page (1,000 strokes, one in ten an erase stroke), deterministic, for the
lasso's timing scenario (z_lasso_3_dense_timing). Never touches a user's data root."""
import math,pathlib,struct,sys
root=pathlib.Path(sys.argv[1]);book=root/'notebooks'/'lasso-dense'
book.mkdir(parents=True,exist_ok=True)
(book/'notebook.ini').write_text('# Grimoire notebook\nname=Dense\nfolder=\npaper=0\ncreated=1\nmodified=1\nlast_page=0\nnext_page_id=2\npage_w=1860\npage_h=2480\npages=1\n')
seed=0xD0D0
def rnd():
    global seed
    seed=(seed*1664525+1013904223)&0xFFFFFFFF
    return seed>>8
def frand(lo,hi): return lo+(hi-lo)*(rnd()%10001)/10000.0
W,H=1860,2480
out=bytearray(struct.pack('<4sHHIII',b'QPG1',1,0,W,H,1000))
for k in range(1000):
    erase=rnd()%10==0
    tool=3 if erase else [2,0,6,7][rnd()%4]
    size=(2.0,2.0) if erase else (0.3,0.6)
    params=[size[0],size[1],0.8,1.0,0.0,1.0,1.0,0.0,70.0,0.0,0.0,0.5,0.5]
    s=(rnd()&0xFFFFFF)
    x,y=frand(0,W),frand(0,H);ang=frand(0,6.28)
    n=10+rnd()%120
    pts=bytearray()
    for i in range(n):
        pts+=struct.pack('<ffHhhI',x,y,int(frand(0.3,1.0)*65535),0,0,i*6)
        ang+=frand(-0.4,0.4);x+=math.cos(ang)*4;y+=math.sin(ang)*4
        x=min(max(x,-100),W+100);y=min(max(y,-100),H+100)
    out+=struct.pack('<B3sI13fI',tool,b'\0\0\0',s,*params,n)+pts
(book/'p1.qpg').write_bytes(bytes(out))
with (root/'settings.ini').open('a') as f:f.write('\nmirror_pdf=0\nstatus_mode=1\n')
