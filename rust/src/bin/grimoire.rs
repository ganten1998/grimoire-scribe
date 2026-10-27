//! The Kindle app (../Makefile's build/kindle/grimoire): main.c's main, in mainloop.rs.
fn main() {
    // (args_os: the C takes any bytes as an argument, where std::env::args would panic on one that
    // is not UTF-8; only --version, --help and -h mean anything, and no other bytes read as them)
    let args = std::env::args_os().map(|a| a.to_string_lossy().into_owned()).collect();
    std::process::exit(grimoire::mainloop::main(args));
}
