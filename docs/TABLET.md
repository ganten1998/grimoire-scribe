# The pen as a Mac tablet (osu!)

`tools/scribe-tablet.c` makes the Kindle Scribe's pen an absolute tablet for a Mac, over Wi-Fi:
the pointer follows the pen where it hovers, and a touch clicks. It was made for osu!, and works
for anything that takes a mouse. It is a Mac tool, apart from the Kindle app: nothing is
installed on the Kindle. It logs in to KOReader's SSH server and runs `cat` on the pen's input
device there.

```
make tablet                      # build/host/scribe-tablet (clang, macOS only)
tests/tablet_check.sh            # builds it and checks it against recorded streams
build/host/scribe-tablet --host 192.168.1.40 -- -i ~/.ssh/kindle_ed25519
```

## Setup

1. **KOReader's SSH server.** In KOReader, turn Wi-Fi on and start Network > SSH server (port
   2222). Put your public key in `koreader/settings/SSH/authorized_keys`. The tool logs in with a
   key and never asks for a password: a prompt would hang its reconnecting.
2. **Accessibility.** macOS drops the pointer events of a program it does not trust, without
   telling it. The trust belongs to the terminal app the tool runs in (Terminal, iTerm,
   Ghostty...). The first start asks for it: allow the terminal in System Settings, Privacy &
   Security, Accessibility, then start the tool again. Until then it stops at once and says so.
3. **Start it.** `scribe-tablet --host ADDR`, where ADDR is the Kindle's address (KOReader shows
   it in Network > Network info). Whatever follows `--` goes to ssh, before the tool's own options,
   so yours win: a key, a known-hosts file, `-F /dev/null` to leave your ssh config out. A Kindle
   whose address changes keeps its host key; `-o HostKeyAlias=kindle` files it under one name:

   ```
   scribe-tablet --host 192.168.1.40 -- -F /dev/null -i ~/.ssh/kindle_ed25519 \
       -o IdentitiesOnly=yes -o UserKnownHostsFile=~/.ssh/kindle_known_hosts -o HostKeyAlias=kindle
   ```

   It prints the area it uses, then `reading /dev/input/event3 (WacomDigitizer)` once the stream
   runs. Ctrl-C stops it (a button it holds is let go first). When the connection ends it tries
   again after 1 s, then 2, 4 and 8 while it keeps failing.
4. **Check the device** with `--list-devices`, which prints the Kindle's input devices by name
   and node.

**`--bind ADDR`.** A Mac with Ethernet and Wi-Fi on the same network may send to the Kindle
through the slower of the two. `--bind` gives ssh the Mac's address on the one to use (`ssh -b`);
macOS then routes through the interface that holds it. ssh's `-B en0` after `--` names the
interface instead.

## Which pen device

The Scribe has two pen devices, and the tool reads the raw one by default:

| | WacomDigitizer (default) | stylus-custom |
|---|---|---|
| what | the digitizer itself: 0..15624 x 0..20832, pressure 0..4095, tilt | a pen service's copy, scaled to the screen: 0..1860 x 0..2480 |
| hover | streams where the pen hovers | says only that the pen is near: no place until it touches |
| pressure | yes | none |
| while KOReader runs | free: KOReader never opens it | grabbed by KOReader: nothing for anyone else |
| timing | first | relayed from the raw one, about 0.1 ms later |

osu! aims with the hovering pen, so only the raw digitizer will do. `--device stylus-custom` (or
any device by name, or `--device event6`) is there for a Kindle where it is the only pen, with
`--range X,Y` for a device the tool does not know.

**Why KOReader holds the scaled pen.** KOReader finds its devices with FBInk's scan
(`frontend/device/kindle/device.lua`), asking for a tablet whose maxima are the framebuffer's
size. That is the scaled pen (1860 x 2480); the raw digitizer's 15624 x 20832 is not it. Every
device KOReader opens it grabs with EVIOCGRAB (koreader-base `input/input.c`, unless
`KO_DONT_GRAB_INPUT` is set, which the Kindle's `koreader.sh` does not do), so another reader of
the scaled pen gets nothing while KOReader runs. Grimoire grabs the scaled pen too, and reads the
raw digitizer without a grab, so the tool works while Grimoire runs as well (the pen then also
draws in Grimoire).

**KOReader still sees the pen.** The pen service feeds the scaled pen from the raw digitizer
whatever else reads it, so a touch is also a tap in KOReader: on a book page it turns the page,
and at the top edge it opens the menu. Before playing, leave KOReader in its file browser on an
empty folder, keep the pen area away from the top edge, or play hovering (`--hover-only`, below),
which never touches. Each redraw of the panel costs the Kindle some CPU, which the stream shares.

**KOReader's sleep.** KOReader puts the Kindle to sleep after its idle time. A hovering pen is
not input KOReader sees, so a session of hover play can reach it; sleep turns Wi-Fi off and
ends the stream (the tool connects again after the wake). Lengthen or turn off KOReader's
autosuspend for the session.

## The pen area and the screen

The pen area maps onto a screen rectangle; everything outside the area holds the pointer at the
screen's edge.

- `--rotate DEG`: how the Kindle is held, turned clockwise from portrait. At 90 its top edge is
  on the right; at 270 on the left. The area is given as the Kindle is held.
- `--area W[,H[,X,Y]]`: the area in millimetres: width, height and centre. Without a height it
  takes the screen's shape; without a centre it sits in the panel's middle. Without `--area` it
  is the largest area of the screen's shape: on a 16:10 screen 157.5 x 98.4 mm held upright, or
  210.0 x 131.2 mm turned (`--rotate 90` or `270`). An area of another shape than the screen's
  works, and the pointer then moves faster one way (the tool says so).
- `--panel-mm W,H`: the panel the digitizer covers, upright. The default, 157.48 x 209.97 mm, is
  the 2022 and 2024 Scribes' 1860 x 2480 px at 300 dpi. The Scribe 3 and the Colorsoft have
  1986 x 2648 px (168.15 x 224.19 mm); their raw digitizer's range is not known here
  (`--list-devices`, then `--range`).
- `--display N` picks a display (0, the default, is the main one); `--screen X,Y,W,H` a rectangle
  in points instead, in the global coordinates (y down) that a second display's position in
  System Settings gives.
- `--flip-x`, `--flip-y`: the raw axes lie as the screen's on the Scribe 2 (Grimoire's log says
  `x as is, y as is` once Live view has compared them); these are for a Kindle where one does not.

An osu! player's area, 100 x 62.5 mm in the middle, on a Scribe held sideways:
`scribe-tablet --host ADDR --rotate 90 --area 100,62.5`.

## Clicks

- A touch is the left button, by the pen's own touch (BTN_TOUCH). `--pressure N` clicks at a
  raw pressure of N instead (0..4095), and lets go below three quarters of it, so a pressure near
  the threshold does not chatter.
- `--right MODE`: a touch is the right button with the side button held (`side`), with the
  eraser end (`eraser`), either (`both`, the default) or never (`none`).
- Taps within 500 ms and 4 points of each other count as a double (and triple) click.
- `--hover-only`: the pen never clicks, it only moves the pointer. For osu!'s hover play, where
  the keys click (osu!'s own K1 and K2, Z and X by default): nothing more is needed from the tool.

## osu! settings

- **osu!lazer**: Settings, Input, Mouse: turn High precision mouse off. With it on, lazer reads
  the pointer's motion and applies its own sensitivity, not the place the pen gives. Its Tablet
  section is for tablets it drives itself; leave it alone.
- **osu!stable**: Options, Input: Raw input off, for the same reason. For hover play turn on
  Disable mouse buttons during play (or use `--hover-only`).
- Either: the cursor size and the keys are yours. macOS's pointer speed and acceleration do not
  apply: the events carry a place, not a motion.

## Latency

From the pen to the pointer:

1. The raw digitizer sends a frame every 2 to 5 ms (measured for Grimoire's Live view).
2. On the Kindle, `cat` writes each read at once and dropbear sends it at once: its sockets have
   TCP_NODELAY, so frames are not held back to fill a packet. The tool asks for no compression,
   which would hold data back too, and marks what the Mac sends as Expedited Forwarding
   (`IPQoS=ef`; OpenSSH 10 takes `lowdelay` no more).
3. **Wi-Fi: almost all of it.** Kindle-to-Mac pings on the same Scribe (2026-10-28): on 5 GHz,
   90% within 7.2 ms and none over 11.7 ms, there and back, so about half that one way. On
   2.4 GHz (channel 11) its link stopped passing frames for about 250 ms of every 330 ms, which
   is unplayable. Check the band on the Kindle with `iw dev wlan0 link` (a frequency of 5000 MHz
   or more is 5 GHz); put the Mac on Ethernet or 5 GHz too.
4. On the Mac, reading and mapping cost about 3 ns an event and 10 to 20 ns a frame
   (`--bench 20000000`, Apple silicon), and making a pointer event about 0.1 us. Each frame is
   posted as soon as its SYN_REPORT is read: no smoothing, no waiting for more.

So on 5 GHz expect a few milliseconds one way, with a few more of jitter now and then; a USB
tablet is 1 to 5 ms. `--stats` prints, every 5 s, the frames per second and how much later than
the quickest of those 5 seconds the frames came (median, p99, max), with drops: compare places
and bands with it. (The Kindle's and the Mac's clocks are not compared: only the spread is.)

**After a stall.** While Wi-Fi stalls the frames wait in the Kindle's buffers and arrive
together after it: the pointer runs through the path it missed, quickly. Only when `cat` itself
cannot keep up (the kernel keeps 128 events for it, 50 to 100 ms of frames) does the kernel drop
events (SYN_DROPPED); a lift may be among them. The tool then keeps a held button down until a
frame says: a pressure (above 0 touches) or the pen coming back. It cannot ask the device what it
holds, as a program on the Kindle would.

## Testing without the Kindle

- `--dry-run` prints the events instead of posting them, one a line: the event's time (the
  Kindle's clock), `move`, `down`, `drag` or `up`, the button, the place in points, and for down
  and up the click count.
- `--capture FILE` keeps the stream as it came; `--replay FILE` plays one back (in real time,
  or at once with `--dry-run`).
- `--bench N` times reading and mapping N synthetic events.
- `tests/tablet_check.sh` builds the tool, replays the streams `tests/tablet/make_capture.py`
  writes (connected mid-stroke, hover, a dipping pressure, double and triple taps, the side
  button, the eraser, two drops, the pen off the area, a stream ending mid-touch and in the middle
  of an event) and compares the events with `tests/tablet/expect-*.txt`, worked out by hand. It
  replays one through a pipe in 13-byte pieces, runs the live path against `tests/tablet/ssh`, a
  stand-in that runs the remote command on the Mac (the device found by name, the reconnect),
  and checks that bad options are refused. It is not part of `make check`.

## Limits

- Pointer events only: the pen's pressure and tilt go nowhere (osu! takes none).
- One tool per Kindle: a second one's start stops the first one's `cat`.
- The raw digitizer's range and the panel size are the 2022 and 2024 Scribes'; another Kindle
  needs `--range` and `--panel-mm`.
- Over SSH and Wi-Fi only, from a Mac.
