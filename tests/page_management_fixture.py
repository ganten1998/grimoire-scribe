"""Named/unlinked pages, stale deleted targets, and colliding notebook page IDs."""
from pathlib import Path
import runpy,struct,sys
runpy.run_path(str(Path(__file__).with_name('notebook_links_fixture.py')),run_name='__main__')
b=Path(sys.argv[1])/'notebooks/A'
p=b/'notebook.ini';s=p.read_text().replace('next_page_id=3','next_page_id=100').replace('pages=1,2\n','pages='+','.join(map(str,range(1,16)))+'\n');p.write_text(s)
for i in range(3,17):
 (b/f'p{i}.qpg').write_bytes(struct.pack('<4sHHIII',b'QPG1',1,4,1860,2480,0))
with (b/'nav.ini').open('a') as f:f.write('name.13=Unlinked thirteen\nname.14=Unlinked fourteen\nname.16=Deleted page\n')
with (b/'links.ini').open('a') as f:f.write('link.2=00000006,p1\narea.2=00000006,200,500,400,200\nlink.13=00000003,p99\narea.13=00000003,200,500,400,200\nlink.14=00000004,n:C:p99\narea.14=00000004,200,500,400,200\nlink.16=00000005,p1\narea.16=00000005,200,500,400,200\n')
(b/'graph.ini').write_text('gen=00000001\nmode=all\nunlinked=show\npos.13=20,30\npos.14=40,60\npos.16=90,20\n')

# A listed but unavailable destination preview must not create an endless retry loop.
(Path(sys.argv[1])/'notebooks/B/p3.qpg').unlink()
