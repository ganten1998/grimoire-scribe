//! The Kindle app (../Makefile's build/kindle/grimoire): main.c's main, in mainloop.rs.
fn main() {
    std::process::exit(grimoire::mainloop::main(std::env::args().collect()));
}
