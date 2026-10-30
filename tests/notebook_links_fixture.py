"""Small deterministic notebooks with colliding page IDs, nested folders and mixed destinations."""
from pathlib import Path
import struct
import sys
root=Path(sys.argv[1])
for ident,name,folder,count in [('A','Origin','Void',2),('B','Destination B','Void/Nested',3),('C','Destination C','Other',2)]:
    book=root/'notebooks'/ident;book.mkdir(parents=True,exist_ok=True)
    (book/'notebook.ini').write_text(f'name={name}\nfolder={folder}\npaper=4\ncreated=1\nmodified=1\nlast_page=0\nnext_page_id={count+1}\npage_w=1860\npage_h=2480\npages='+','.join(str(i) for i in range(1,count+1))+'\n')
    for i in range(1,count+1):
        (book/f'p{i}.qpg').write_bytes(struct.pack('<4sHHIII',b'QPG1',1,4,1860,2480,0))
    (book/'nav.ini').write_text(f'name.2={name} second page\n')
(root/'notebooks/A/links.ini').write_text('link.1=00000001,p2\narea.1=00000001,300,500,400,200\nlink.1=00000002,n:C:p2\narea.1=00000002,300,900,400,200\n')
(root/'shortcuts.ini').write_text('key.b=go-back\nkey.x=close-menu\n')
with (root/'settings.ini').open('a') as f:f.write('\nshortcuts_enabled=1\nmirror_pdf=0\n')
