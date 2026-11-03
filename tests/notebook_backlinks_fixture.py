"""tests/notebook_links_fixture.py's notebooks, then for the links-between-notebooks polish (2.0.0-beta.2):
B links to C's second page too (so C's page 2 is linked from two notebooks), the links bar has a key, and
with "broken" before the root (# generate: tests/notebook_backlinks_fixture.py broken) C's page 2 is deleted and A's page 2 links to a notebook that is gone.
With "many", 120 more notebooks (folder Many, ids 0000 to 0119: checked before A, B and C), each with a links.ini, so
the cache's first check takes 120 idle ticks or more. With "back", A's page 2 links to B's page 1."""
from pathlib import Path
import struct
import subprocess
import sys
root=Path(sys.argv[-1])
subprocess.run([sys.executable,str(Path(__file__).with_name('notebook_links_fixture.py')),str(root)],check=True)
(root/'notebooks/B/links.ini').write_text('link.3=00000005,n:C:p2\narea.3=00000005,300,500,400,200\n')
with (root/'shortcuts.ini').open('a') as f:f.write('key.l=links\n')
if 'broken' in sys.argv[1:-1]:
    ini=root/'notebooks/C/notebook.ini'
    ini.write_text(ini.read_text().replace('pages=1,2','pages=1'))
    (root/'notebooks/C/p2.qpg').unlink()
    with (root/'notebooks/A/links.ini').open('a') as f:
        f.write('link.2=00000009,n:Gone_notebook:p1\narea.2=00000009,300,500,400,200\n')
if 'many' in sys.argv[1:-1]:
    for k in range(120):
        book=root/'notebooks'/f'{k:04d}';book.mkdir(parents=True,exist_ok=True)
        (book/'notebook.ini').write_text(f'name=Many {k}\nfolder=Many\npaper=4\ncreated=1\nmodified=1\nlast_page=0\nnext_page_id=2\npage_w=1860\npage_h=2480\npages=1\n')
        (book/'p1.qpg').write_bytes(struct.pack('<4sHHIII',b'QPG1',1,4,1860,2480,0))
        (book/'links.ini').write_text('link.1=00000001,p1\n')
if 'back' in sys.argv[1:-1]:
    with (root/'notebooks/A/links.ini').open('a') as f:
        f.write('link.2=0000000a,n:B:p1\narea.2=0000000a,300,500,400,200\n')
