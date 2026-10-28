# Working on Grimoire in a Claude Code cloud session

Read this first in a cloud session (claude.ai/code or `claude --cloud`).

## Set up the machine (Ubuntu 24.04 x86_64, root)

Run this once at the start; it takes a few minutes. Use the background for long downloads if you like.

```
set -e
apt-get update -qq && apt-get install -y -qq gettext xxd clang make python3-pip nodejs >/dev/null || true
if ! command -v rustup >/dev/null 2>&1; then
  curl -sSf https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init -o /tmp/rustup-init
  chmod +x /tmp/rustup-init && /tmp/rustup-init -y --profile minimal --default-toolchain 1.97.1
fi
. "$HOME/.cargo/env"
rustup toolchain install 1.97.1 --profile minimal && rustup default 1.97.1
rustup target add armv7-unknown-linux-musleabihf
pip install --break-system-packages ziglang==0.16.0 || pip install --break-system-packages ziglang
cargo install cargo-zigbuild --locked
```

- cargo-zigbuild finds zig through the `ziglang` Python package. The Makefile's `rust-kindle` target uses
  `cargo zigbuild`.
- The Mac-specific PATH note in `rust/PORTING.md` (Homebrew's rustc) does not apply here.

## Build and test

- From `rust/`: `cargo build`, `cargo build --features sim`, `cargo test -- --test-threads=1`.
- From the top: `GRIMOIRE_TEST_PORT=8850 make rust-test`, `make rust-scenarios`, `make rust-l10n-check`,
  `make rust-kindle`, and `make rust-check` (the gate).

**This machine is Linux x86_64. The owner's Mac (arm64) is where the authoritative gate runs.** Expect a few
host-specific results:

- recorded float fixtures, such as the brush probe fixture recorded with Apple's libm;
- outline-font CRCs, which are compared on Apple arm64 only.

Do not change a test's expectation to make it pass on Linux. Say which ones differ and why, and leave them for the
Mac gate.

## Rules

- `rust/PORTING.md` is the contract, and `docs/index-hook/README.md` holds the owner's rulings.
- No device access, ever (no ssh; the Kindle is on the owner's LAN anyway).
- Never run a GUI binary.
- No em dashes in code comments, docs or UI.
- Every UI string must exist in English and Spanish: update `po/es.po` and run `make l10n`, then rebuild the .mo
  with `msgfmt --check --no-hash --endianness=little -o po/es.mo po/es.po`. Strings must fit both orientations.
- Never change the `.qpg` page format (no header version bump, no tool id >= 10).
- Never add an MXCFB_WAIT_FOR_UPDATE_SUBMISSION wait or a second wait on one display marker. The Kindle's e-ink
  driver panics the kernel on the first and stalls 5 s on the second.
- The main loop must never block ~100 ms (the pen's evdev buffer overflows). Long work pumps input.
- Commit in small logical commits on your own branch (named in your task), each message ending with:

```
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01Uoe9ZN5YbYueDV4wnuVtnD
```

- Push your branch when done (and at checkpoints).
- Never push to `main`, `claude/rust-port` or `claude/beta5-features`. The owner's local session reviews and merges.
