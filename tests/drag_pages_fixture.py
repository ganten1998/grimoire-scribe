"""Drag a held page to a gap: a twelve-page notebook (more than one screenful of the Pages view's nine),
page k named with its number in words."""
from pathlib import Path
import struct
import sys
root = Path(sys.argv[1])
book = root / 'notebooks' / 'A'
book.mkdir(parents=True, exist_ok=True)
n, w, h = 12, 1860, 2480
(book / 'notebook.ini').write_text(f'name=Dragged\nfolder=Void\npaper=4\ncreated=1\nmodified=1\nlast_page=0\n'
                                   f'next_page_id={n + 1}\npage_w={w}\npage_h={h}\npages='
                                   + ','.join(str(i) for i in range(1, n + 1)) + '\n')
for i in range(1, n + 1):
    (book / f'p{i}.qpg').write_bytes(struct.pack('<4sHHIII', b'QPG1', 1, 4, w, h, 0))
words = ['one', 'two', 'three', 'four', 'five', 'six', 'seven', 'eight', 'nine', 'ten', 'eleven', 'twelve']
(book / 'nav.ini').write_text(''.join(f'name.{i + 1}={wd}\n' for i, wd in enumerate(words)))
with (root / 'settings.ini').open('a') as f:
    f.write('\nmirror_pdf=0\n')
