//! A streamed zip that would pass 4 GiB (2.1.0-beta.2 fix round, M15): the format without zip64 cannot say
//! where its entries are past that, and what is already sent cannot be taken back, so the whole zip fails:
//! an entry refused fails it, and so does a central directory past 32 bits (0c26d912 left the entries out
//! one by one, wrapped the directory's offset and called the archive good). The bytes go to a count.

use grimoire::prelude::*;

#[test]
fn a_zip_past_4_gib_fails() {
    let mut scratch = Buf::default();
    let mut count = CountSink::default();
    let chunk = vec![0x5Au8; 1 << 20];
    // (2,200 MiB an entry: each is under the limit, two are over it)
    let mut fill = |s: &mut dyn Sink| {
        for _ in 0..2200 {
            if !s.put(&chunk) {
                return -1;
            }
        }
        0
    };
    let (a, b, raw, c, end);
    {
        let mut z = zip_begin_to(&mut scratch, &mut count);
        a = zip_add_stream(&mut z, "A/A.pdf", &mut fill);
        b = zip_add_stream(&mut z, "B/B.pdf", &mut fill);
        raw = zip_add(&mut z, "B/raw/notebook.ini", b"name=B\n", false);
        c = zip_add_stream(&mut z, "C/C.pdf", &mut fill);
        end = zip_end(&mut z);
    }
    assert_eq!((a, b), (0, 0), "each entry starts within 32 bits");
    assert_eq!((raw, c), (-1, -1), "what comes after the limit is refused");
    assert!(count.0 > u32::MAX as u64);
    assert_eq!(end, -1, "the zip is no good, and says so");
    // an entry whose maker fails midway (a notebook's PDF in a folder's zip, out of memory or disk): its header
    // and first bytes are out, so the zip fails (0c26d912 answered it as good, the entry orphaned in it)
    let mut scratch = Buf::default();
    let mut out = Buf::default();
    let mut sink = BufSink(&mut out);
    let mut z = zip_begin_to(&mut scratch, &mut sink);
    assert_eq!(zip_add(&mut z, "a.txt", b"one", false), 0);
    assert_eq!(zip_add_stream(&mut z, "b.pdf", &mut |s: &mut dyn Sink| {
        let _ = s.put(b"half");
        -1
    }), -1);
    assert_eq!(zip_end(&mut z), -1, "a broken entry fails the zip");
}

