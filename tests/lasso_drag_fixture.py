#!/usr/bin/env python3
"""A notebook with one page of pencil shading, deterministic, for the lasso drag scenarios
(z_lasso_19_drag_dense, z_lasso_20_drag_live): three hexagons coloured in with 12 Pencil strokes of 9,852
points in all (300 to 4,381 each), about 1 to 1.8 px apart, in a box of about 168 x 166 px at (493, 487).
These are the statistics of the 12 strokes the owner copied, pasted and dragged on his Scribe (2026-11-05,
page 5 of his notebook): generated here, never his drawing. Hexagon paper, as his page. Never touches a
user's data root."""
import math,pathlib,struct,sys
root=pathlib.Path(sys.argv[1]);book=root/'notebooks'/'lasso-shading'
book.mkdir(parents=True,exist_ok=True)
(book/'notebook.ini').write_text('# Grimoire notebook\nname=Shading\nfolder=\npaper=19\ncreated=1\nmodified=1\nlast_page=0\nnext_page_id=2\npage_w=1860\npage_h=2480\npages=1\n')
seed=0x5EED
def rnd():
    global seed
    seed=(seed*1664525+1013904223)&0xFFFFFFFF
    return seed>>8
def frand(lo,hi): return lo+(hi-lo)*(rnd()%10001)/10000.0
W,H=1860,2480
# (centre x, centre y, circumradius) of three flat-topped hexagons, side by side as on a hexagon page
HEX=[(536,572,40),(617,527,44),(602,617,42)]
def span(h,y):
    cx,cy,r=h;dy=abs(y-cy);top=r*math.sqrt(3)/2
    if dy>top: return None
    hw=r-dy/math.sqrt(3)
    return cx-hw,cx+hw
def scribble(h,n,row0,step):
    """n points walking rows across hexagon h, back and forth, bouncing at its top and bottom: rows 2 px apart
    for a stroke with points enough to go over it more than once, wider (up to 8 px) for one that colours
    it in once."""
    cx,cy,r=h;top=cy-r*math.sqrt(3)/2+1;bot=cy+r*math.sqrt(3)/2-1
    gap=min(8,max(2,round((bot-top)*1.5*r/(n*step))))
    pts=[];y=top+row0%max(1,int(bot-top));dirn=1;right=row0%2==0
    while len(pts)<n:
        s=span(h,y)
        if s:
            x0,x1=s
            xs=[x0+k*step for k in range(int((x1-x0)/step)+1)]
            if not right: xs.reverse()
            for x in xs:
                if len(pts)>=n: break
                pts.append((x+frand(-0.4,0.4),y+frand(-0.4,0.4)))
            right=not right
        y+=gap*dirn
        if y>bot: y=bot;dirn=-1
        if y<top: y=top;dirn=1
    return pts
COUNTS=[4381,1300,800,600,450,400,350,330,320,310,305,306]
PARAMS=[0.12,0.32,0.85,0.95,0.0,1.0,1.0,25.0,55.0,4.0,0.75,0.75,0.0]
# version 2, for its 32-bit paper: 19, Hexagon (the u16 at 6 says Blank, as docs/FORMAT.md has it)
out=bytearray(struct.pack("<4sHHIIII",b"QPG1",2,4,W,H,19,len(COUNTS)))
t=1000
for k,n in enumerate(COUNTS):
    h=HEX[0] if k==0 else HEX[k%3]
    pts=scribble(h,n,(k*7)%20,1.2 if k else 1.5)
    s=(rnd()&0xFFFFFF)
    out+=struct.pack('<B3sI13fI',0,b'\0\0\0',s,*PARAMS,n)
    for i,(x,y) in enumerate(pts):
        out+=struct.pack('<ffHhhI',x,y,int(frand(0.15,0.45)*65535),2900,4000,t)
        t+=3
    t+=400
(book/'p1.qpg').write_bytes(bytes(out))
