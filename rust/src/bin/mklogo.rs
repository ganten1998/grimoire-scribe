//! tools/mklogo.c: the logo for the README and releases (docs/logo.png, docs/logo-dark.png), black
//! and white on transparent: committed, like the icon, and run by hand (../Makefile's logo rule).
// The skeleton stage (PORTING.md, Finishing): bodies are todo!().
#![allow(unused_variables, unused_imports, dead_code)]
use grimoire::prelude::*;

fn main() {
    std::process::exit(run(std::env::args().collect()));
}

fn run(args: Vec<String>) -> i32 {
    todo!()
}
