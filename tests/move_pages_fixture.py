"""Move to... and Copy to notebook...: a named five-page notebook with links, a second notebook of the
other Scribe's page size to copy into, and a third elsewhere."""
from pathlib import Path
import struct
import sys
root = Path(sys.argv[1])
books = [('A', 'Origin', 'Void', 5, 1860, 2480), ('B', 'Destination B', 'Void', 2, 1986, 2648), ('C', 'Elsewhere', 'Other', 1, 1860, 2480)]
for ident, name, folder, count, w, h in books:
    book = root / 'notebooks' / ident
    book.mkdir(parents=True, exist_ok=True)
    (book / 'notebook.ini').write_text(f'name={name}\nfolder={folder}\npaper=4\ncreated=1\nmodified=1\nlast_page=0\n'
                                       f'next_page_id={count + 1}\npage_w={w}\npage_h={h}\npages='
                                       + ','.join(str(i) for i in range(1, count + 1)) + '\n')
    for i in range(1, count + 1):
        (book / f'p{i}.qpg').write_bytes(struct.pack('<4sHHIII', b'QPG1', 1, 4, w, h, 0))
words = ['one', 'two', 'three', 'four', 'five']
(root / 'notebooks/A/nav.ini').write_text(''.join(f'name.{i + 1}={wd}\n' for i, wd in enumerate(words)))
(root / 'notebooks/B/nav.ini').write_text('name.1=B first\nname.2=B second\n')
(root / 'notebooks/A/links.ini').write_text(
    'link.1=00000001,p3\narea.1=00000001,300,500,400,200\n'
    'link.1=00000002,index\narea.1=00000002,300,900,400,200\n'
    'link.3=00000003,n:B:p1\narea.3=00000003,300,500,400,200\n'
    'link.2=00000004,p4\narea.2=00000004,300,500,400,200\n')
with (root / 'settings.ini').open('a') as f:
    f.write('\nmirror_pdf=0\n')
