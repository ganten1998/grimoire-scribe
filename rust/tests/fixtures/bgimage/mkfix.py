# mkfix.py: makes the pictures in this folder, which tests/test_bgimage.rs and bgimage_probe.c read.
#   python3 mkfix.py <this folder>
# PNGs are written by hand (every bit depth and colour type stb_image reads, a real Adam7 one, an
# iPhone CgBI one, headers that claim sizes a picture may not have, a zlib bomb, damaged ones), PNM
# likewise; the JPEGs come from libjpeg's cjpeg (baseline and progressive, 4:2:0, 4:2:2, 4:4:4, gray)
# and the exif_* files are a small JPEG with an Exif block (both byte orders, orientation 0 to 9)
# put after its start. Everything is seeded, so the files come out the same each time.
import zlib, struct, random, os, subprocess, sys, tempfile
out = sys.argv[1]
tmp = tempfile.mkdtemp()
random.seed(12345)

def chunk(t, d):
    c = struct.pack(">I", len(d)) + t + d
    return c + struct.pack(">I", zlib.crc32(t + d) & 0xffffffff)

def png(w, h, depth, color, rows, extra=b"", interlace=0, raw_idat=None):
    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", w, h, depth, color, 0, 0, interlace)
    if raw_idat is None:
        raw = b"".join(b"\x00" + r for r in rows)
        idat = zlib.compress(raw, 6)
    else:
        idat = raw_idat
    return sig + chunk(b"IHDR", ihdr) + extra + chunk(b"IDAT", idat) + chunk(b"IEND", b"")

def rnd(n): return bytes(random.randrange(256) for _ in range(n))

def rows_of(w, h, bpp, fn=None):
    return [rnd(w * bpp) if fn is None else fn(y) for y in range(h)]

def save(name, data):
    with open(os.path.join(out, name), "wb") as f: f.write(data)

# 8-bit
save("gray8.png", png(32, 24, 8, 0, rows_of(32, 24, 1)))
save("rgb8.png", png(30, 22, 8, 2, rows_of(30, 22, 3)))
save("rgba8.png", png(25, 20, 8, 6, rows_of(25, 20, 4)))
save("ga8.png", png(17, 11, 8, 4, rows_of(17, 11, 2)))
# an RGB picture with no colour at all (decodes to gray), and one with one coloured pixel
def graylike(y, w=40):
    r = bytearray()
    for x in range(w):
        v = (x * 7 + y * 3) & 255
        r += bytes([v, v, v])
    return bytes(r)
save("grayrgb.png", png(20, 15, 8, 2, [graylike(y, 20) for y in range(15)]))
def graylike1(y, w=40):
    r = bytearray(graylike(y, w))
    if y == 14: r[-1] ^= 1
    return bytes(r)
save("grayrgb1.png", png(20, 15, 8, 2, [graylike1(y, 20) for y in range(15)]))
# 16-bit
save("rgb16.png", png(20, 15, 16, 2, rows_of(20, 15, 6)))
save("gray16.png", png(19, 14, 16, 0, rows_of(19, 14, 2)))
save("rgba16.png", png(10, 9, 16, 6, rows_of(10, 9, 8)))
# palette, with tRNS
plte = bytes(random.randrange(256) for _ in range(3 * 16))
save("pal4.png", png(25, 20, 4, 3, rows_of(25, 20, 1, lambda y: bytes(random.randrange(256) & 0xff for _ in range((25 * 4 + 7) // 8))), extra=chunk(b"PLTE", plte)))
trns = bytes(random.randrange(256) for _ in range(8))
save("pal8trns.png", png(16, 12, 8, 3, rows_of(16, 12, 1, lambda y: bytes(random.randrange(16) for _ in range(16))), extra=chunk(b"PLTE", plte) + chunk(b"tRNS", trns)))
# 1 bit gray, tRNS gray
save("gray1.png", png(35, 10, 1, 0, rows_of(35, 10, 1, lambda y: rnd((35 + 7) // 8))))
# interlaced: header says Adam7 but data is a plain stream (stb will fail or garble; same for both)
save("interlaced_bad.png", png(15, 10, 8, 2, rows_of(15, 10, 3), interlace=1))
# a real Adam7 8-bit RGB
def adam7(w, h, bpp, data):
    passes = [(0,0,8,8),(4,0,8,8),(0,4,4,8),(2,0,4,4),(0,2,2,4),(1,0,2,2),(0,1,1,2)]
    raw = bytearray()
    for (x0,y0,dx,dy) in passes:
        xs = list(range(x0, w, dx)); ys = list(range(y0, h, dy))
        if not xs or not ys: continue
        for y in ys:
            raw.append(0)
            for x in xs:
                raw += data[(y*w+x)*bpp:(y*w+x+1)*bpp]
    return bytes(raw)
w,h=19,15
d = rnd(w*h*3)
raw = adam7(w,h,3,d)
save("adam7.png", png(w,h,8,2,None,interlace=1,raw_idat=zlib.compress(raw)))
# CgBI-led (iPhone) header: a CgBI chunk before IHDR; stb treats it as iphone png (needs flag), info may fail
sig = b"\x89PNG\r\n\x1a\n"
cg = sig + chunk(b"CgBI", b"\x00\x00\x00\x00") + chunk(b"IHDR", struct.pack(">IIBBBBB", 20, 20, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(b"\x00" * (20 * 61))) + chunk(b"IEND", b"")
save("cgbi.png", cg)
# header-only fakes (only the header is read by check): sizes
def fake(w, h, depth, color, interlace=0):
    return png(w, h, depth, color, None, raw_idat=zlib.compress(b""), interlace=interlace, extra=(chunk(b"PLTE", plte) if color == 3 else b""))
save("fake_9000.png", fake(9000, 10, 8, 0))
save("fake_7000sq.png", fake(7000, 7000, 8, 0))
save("fake_5000x4000.png", fake(5000, 4000, 8, 0))
save("fake_rgb16_6000x3000.png", fake(6000, 3000, 16, 2))
save("fake_rgb16_3000x3000.png", fake(3000, 3000, 16, 2))
save("fake_pal_6000x5000.png", fake(6000, 5000, 8, 3))
save("fake_inter_4000x4000.png", fake(4000, 4000, 8, 2, interlace=1))
save("fake_rgba_6000x6000.png", fake(6000, 6000, 8, 6))
save("fake_6000.png", fake(6000, 6000, 8, 0))
save("fake_6001.png", fake(6001, 100, 8, 0))
save("fake_1bit_8000.png", fake(8000, 4000, 1, 0))
# a zlib bomb: 8x8 gray whose stream inflates to 8 MB
bomb = zlib.compress(b"\x00" * (4 * 1024 * 1024), 9)
save("bomb.png", png(8, 8, 8, 0, None, raw_idat=bomb))
# 8x8 whose stream is short (damaged)
save("short.png", png(40, 40, 8, 0, None, raw_idat=zlib.compress(b"\x00" * 100)))
save("trunc.png", png(30, 22, 8, 2, rows_of(30, 22, 3))[:100])
save("junk.bin", rnd(300))
save("tiny.bin", b"abc")
save("empty.bin", b"")

# PNM
def pnm(kind, w, h, maxv, body): return ("P%d\n%d %d\n%d\n" % (kind, w, h, maxv)).encode() + body
save("p5.pgm", pnm(5, 15, 9, 255, rnd(15 * 9)))
save("p6.ppm", pnm(6, 14, 10, 255, rnd(14 * 10 * 3)))
save("p6_16.ppm", pnm(6, 13, 11, 65535, rnd(13 * 11 * 6)))
save("p5_16.pgm", pnm(5, 17, 9, 65535, rnd(17 * 9 * 2)))
save("p6_short.ppm", pnm(6, 14, 10, 255, rnd(50)))
save("p6_100.ppm", pnm(6, 14, 10, 100, rnd(14 * 10 * 3)))

# JPEGs, via cjpeg
def ppm_img(w, h, gray=False):
    px = bytearray()
    for y in range(h):
        for x in range(w):
            r = (x * 255 // max(w - 1, 1) + random.randrange(8)) & 255
            g = (y * 255 // max(h - 1, 1) + random.randrange(8)) & 255
            b = ((x + y) * 3 + random.randrange(8)) & 255
            px += bytes([r, g, b])
    return pnm(6, w, h, 255, bytes(px))
big = ppm_img(67, 39)
with open(os.path.join(tmp, "src.ppm"), "wb") as f: f.write(big)
with open(os.path.join(tmp, "tiny.ppm"), "wb") as f: f.write(ppm_img(24, 16))
def cj(name, *args, src="src.ppm"):
    r = subprocess.run(["cjpeg", *args, os.path.join(tmp, src)], capture_output=True)
    save(name, r.stdout)
cj("base_420.jpg", "-quality", "85")
cj("base_444.jpg", "-quality", "85", "-sample", "1x1")
cj("base_422.jpg", "-quality", "85", "-sample", "2x1")
cj("base_gray.jpg", "-grayscale", "-quality", "85")
cj("prog_420.jpg", "-progressive", "-quality", "85")
cj("prog_444.jpg", "-progressive", "-quality", "85", "-sample", "1x1")
cj("prog_gray.jpg", "-progressive", "-grayscale")
# JPEG with padding bytes after a segment and an Exif block
cj("tiny_420.jpg", "-quality", "85", src="tiny.ppm")
j = open(os.path.join(out, "tiny_420.jpg"), "rb").read()
def tiff(le, orient, extra_tags=True):
    e = "<" if le else ">"
    t = (b"II" if le else b"MM") + struct.pack(e + "H", 42) + struct.pack(e + "I", 8)
    tags = [(0x010F, 2, 1, 0)] if extra_tags else []
    tags.append((0x0112, 3, 1, orient))
    t += struct.pack(e + "H", len(tags))
    for (tag, typ, cnt, val) in tags:
        t += struct.pack(e + "HHI", tag, typ, cnt) + (struct.pack(e + "H", val) + b"\0\0")
    t += struct.pack(e + "I", 0)
    return t
def with_exif(j, tf):
    body = b"Exif\0\0" + tf
    seg = b"\xff\xe1" + struct.pack(">H", len(body) + 2) + body
    return j[:2] + seg + j[2:]
for o in (1, 2, 3, 4, 5, 6, 7, 8):
    save("exif_le_%d.jpg" % o, with_exif(j, tiff(True, o)))
    save("exif_be_%d.jpg" % o, with_exif(j, tiff(False, o, False)))
save("exif_0.jpg", with_exif(j, tiff(True, 0)))
save("exif_9.jpg", with_exif(j, tiff(True, 9)))
save("exif_pad.jpg", j[:2] + b"\x00\x00\x00" + j[2:])   # padding before the first marker
