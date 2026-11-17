**English** | [日本語](../japanese/Privacy-ja.md)

# Privacy

- **The Device Passcode stays on for a Library start.** Grimoire never reads or stores your
  passcode: the Kindle's own keypad takes it, and Grimoire logs no input while the keypad has the
  screen. This hand-over still needs confirmation on each firmware, and KOReader's own session can
  have turned the passcode off; see [Using Grimoire](Using-Grimoire#passcode).
- **Erased ink.** Fully erased strokes are removed by background compaction after a notebook
  closes. A partially erased stroke remains whole in the raw page file; old exporters can show its
  erased parts. Deleted pages and notebooks stay in `grimoire/trash` until you remove them.
- **Share has no password.** Anyone on the same network can open it while the Share screen is
  open, and nobody once you leave it. Do not open Share on a network you do not trust.
- **`grimoire.log`**, which a bug report asks for, holds no serial number, account, Wi-Fi name or
  network address, and Share notes downloads without notebook or folder names. It never names a
  font of yours either, by file or by family: only sizes, counts and timings.
- **Files from outside are read in a sandbox.** A PDF, a picture or a font you copy onto the
  Kindle is first read by a separate, locked-down copy of Grimoire that has no network, sees no
  files, runs as a user of its own and may make only a short list of system calls; what it
  reports is checked before anything is written. A font is drawn by Grimoire itself only after it
  passed that check for exactly its bytes.
- **Live view and Share are plain HTTP**, only while their screen is open, and only on your local
  network. Use them on a network you trust.
- **Wi-Fi passwords never reach Grimoire.** Add or change networks in the Kindle's settings or
  KOReader; Grimoire's helper keeps only the networks' names.
- **Your fonts stay on the Kindle.** Grimoire never copies a font of yours into a page, a PDF or
  anywhere else, and never sends it anywhere.
