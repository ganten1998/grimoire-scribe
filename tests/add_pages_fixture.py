"""New page's batch at the limit: a notebook of 1995 pages (STORE_MAX_PAGES is 2000), only the first page's
file written (a page whose file is not there is a blank page), and a small one beside it."""
from pathlib import Path
import struct
import sys
root = Path(sys.argv[1])
for ident, name, count in [('F', 'Nearly full', 1995), ('S', 'Small', 2)]:
    book = root / 'notebooks' / ident
    book.mkdir(parents=True, exist_ok=True)
    (book / 'notebook.ini').write_text(f'name={name}\nfolder=Void\npaper=4\ncreated=1\nmodified=1\nlast_page=0\n'
                                       f'next_page_id={count + 1}\npage_w=1860\npage_h=2480\npages='
                                       + ','.join(str(i) for i in range(1, count + 1)) + '\n')
    (book / 'p1.qpg').write_bytes(struct.pack('<4sHHIII', b'QPG1', 1, 4, 1860, 2480, 0))
with (root / 'settings.ini').open('a') as f:
    f.write('\nmirror_pdf=0\n')
