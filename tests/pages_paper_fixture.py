"""The Pages view's Redo and the undo and redo keys (2.0.0-beta.2, the owner's rulings of 2026-11-03): a five-page
notebook whose pages are each on a paper of their own (1 blank, 2 wide rule, 3 dots, 4 grid, 5 medium rule; the
notebook's own paper blank; page 2 open), so a batch shows which page's paper it took."""
from pathlib import Path
import struct
import sys
root = Path(sys.argv[1])
papers = [4, 2, 10, 11, 1]
book = root / 'notebooks' / 'A'
book.mkdir(parents=True, exist_ok=True)
(book / 'notebook.ini').write_text('name=Papers\nfolder=Void\npaper=4\ncreated=1\nmodified=1\nlast_page=1\n'
                                   f'next_page_id={len(papers) + 1}\npage_w=1860\npage_h=2480\npages='
                                   + ','.join(str(i) for i in range(1, len(papers) + 1)) + '\n')
for i, paper in enumerate(papers, 1):
    (book / f'p{i}.qpg').write_bytes(struct.pack('<4sHHIII', b'QPG1', 1, paper, 1860, 2480, 0))
with (root / 'settings.ini').open('a') as f:
    f.write('\nmirror_pdf=0\n')
