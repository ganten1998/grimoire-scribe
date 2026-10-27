//! The headless runner (../Makefile's build/host/grimoire-headless): the app driven by a script
//! (GRIMOIRE_SCRIPT=<file>, docs/SIM.md), main.c's main in mainloop.rs with the simulator's platform
//! (sim_sdl.rs). It exits with status 3 when an expect fails, as the C runner does.
fn main() {
    std::process::exit(grimoire::mainloop::main(std::env::args().collect()));
}
