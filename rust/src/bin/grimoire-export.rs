//! tools/grimoire-export.c: render a Grimoire data root (e.g. rsynced off the Kindle) into PDFs / PNGs
//! on a computer, with exactly the same renderer the Kindle uses.
//!
//!   grimoire-export [--png] [--zip] [--force] <grimoire-root> <out-dir>
//!
//! Writes <out>/<folder>/<name>.pdf (and <name>/page-NNN.png with --png, <name>.zip with --zip).
//! Incremental: a notebook is skipped when its PDF is newer than its last modification.
// The skeleton stage (PORTING.md, Finishing): bodies are todo!().
#![allow(unused_variables, unused_imports, dead_code)]
use grimoire::prelude::*;

fn write_out(path: &str, b: &Buf) -> i32 {
    todo!()
}

fn main() {
    std::process::exit(run(std::env::args().collect()));
}

fn run(args: Vec<String>) -> i32 {
    todo!()
}
