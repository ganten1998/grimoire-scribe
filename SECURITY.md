# Security.md
 
## Project Overview
 
Grimoire is a handwriting notebook app for jailbroken Kindle Scribe. It is free software (GPL-3.0-or-later) that functions entirely on-device with no cloud, account, or tracking.
 
## Threat Model

- **Device-local only**: All notebooks, notes, and data remain on the Kindle. No data is transmitted to external servers unless the user explicitly initiates Share or Live view.
- **Jailbreak context**: The app runs on a jailbroken Kindle Scribe. Security relies on the Kindle's own passcode and the sandboxed execution model.
- **No authentication**: Grimoire has no user accounts, passwords, or authentication mechanisms. The only access control is the Kindle's device passcode.
- **Network exposure**: Share and Live view expose the Kindle's screen on the local network via plain HTTP.
 
## Core Security Properties
 
### Device Passcode
- The Kindle's device passcode is managed entirely by the Kindle system. Grimoire never reads or stores the passcode — the Kindle's keypad handles authentication.
- When Grimoire starts, the passcode stays enabled throughout the session. Leaving Grimoire while the Kindle shows its screensaver or passcode prompt no longer pauses the passcode (since 2.0.0-beta.2).
- KOReader normally turns the passcode off before Grimoire can run; starting Grimoire from the Library re-enables it.
- The passcode reset code `111222777` erases data — never use it to exit Grimoire.
 
### Share (Wi-Fi)
- Share has **no password**. Anyone on the same network can open it while the Share screen is open, and nobody once you leave it.
- **Do not open Share on a network you do not trust.**
- Wi-Fi passwords never reach Grimoire — only network names are kept.
- Share uses plain HTTP, only while the screen is open, and only on your local network.
 
### Live View
- Live view and Share are plain HTTP, only while their screen is open, and only on your local network.
- Use them on a network you trust.
 
### Fonts and External Files
- Your fonts stay on the Kindle. Grimoire never copies a font into a page, PDF, or anywhere else, and never sends them anywhere.
- Files from outside (PDFs, pictures, fonts) are read in a sandbox: a separate, locked-down copy of Grimoire that has no network access, sees no files, runs as its own user, and may make only a short list of system calls. What it reports is checked before anything is written.
- A font is drawn by Grimoire itself only after it passed that check for exactly its bytes.
- The font check limits: glyph processing timed at 160px, free floor at 4MB storage, 512 files floor.
 
### Erased Ink
- Fully erased strokes are removed by background compaction after a notebook closes.
- A partially erased stroke remains whole in the raw page file; old exporters can show its erased parts.
- Deleted pages and notebooks stay in `grimoire/trash` until you remove them.
 
### Logs (`grimoire.log`)
- Holds no serial number, account, Wi-Fi name or network address.
- Share notes downloads contain no notebook or folder names.
- Never names a font by file or family — only sizes, counts and timings.
 
## Security Practices
 
### Dependencies
- Dependencies are audited via `cargo deny` (run in CI). The `deny.toml` bans `glifo` (vello_cpu's optional text renderer) and limits licenses to permissive ones (MIT, Apache-2.0, BSD-3-Clause, Zlib, Unicode-3.0, Unlicense, 0BSD, GPL-3.0-or-later).
- `multiple-versions` and `wildcards` are banned; known duplicates are listed in `skip` with reasons.
- No unknown registries or git sources allowed — all crates must come from crates.io (the committed Cargo.lock index).
 
### Input / File Handling
- Font files have a maximum size limit (`FONT_FILE_MAX`) before they are refused.
- Font path names are scanned: names starting with `.` are rejected as fonts.
- The font copy job writes only into Grimoire's own folders (`import`, `templates`, `photos`, `fonts`), never at the storage root.
- Write zones are allow-listed: only folders below Grimoire's own folder are writable.
- A font file must pass the check (including size limits) before any bytes are copied or it becomes usable.
 
### Sandboxing
- The sandbox runs on the Kindle with restricted system calls (`sandbox.rs`). The exact system-call rules differ between the host build and the Kindle build (`s1` rules).
- Sandboxed steps are hardened round 2026-11 and documented in `docs/TESTING.md` under "Hardening round (2026-11), sandbox".
 
### Error Handling
- Panic mode is `abort` (never unwind), set in `Cargo.toml` for release builds.
- Debug overflow checks are on (`overflow-checks = true` in dev profile).
- Release builds strip symbols (`strip = true`), use LTO (`lto = true`), and optimize at level 2.
 
## Reporting Security Issues
 
Report security vulnerabilities privately by emailing the project maintainers. Do not publicly disclose issues until they have been addressed.
 
## Compliance
 
- **GPL-3.0-or-later**: All modifications must be released under the same license.
- **On-device only**: No user data leaves the Kindle unless the user explicitly chooses Share or Live view.
- **No tracking**: Grimoire contains no analytics, telemetry, or remote identifier.
 
Last updated: November 2026
