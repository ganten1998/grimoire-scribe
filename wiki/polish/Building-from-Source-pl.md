[English](Building-from-Source) | [日本語](Building-from-Source-ja) | **Polski**

# Budowanie ze źródeł

Aplikacja jest w Rust: jeden crate w `rust/` (`rust/Cargo.toml`; na górze nie ma `Cargo.toml`),
budowany i sprawdzany przez główny Makefile. Uruchamiaj cargo w `rust/` albo używaj make;
`rust/PORTING.md` ma polecenia cargo i zasady dla kodu.

## Czego potrzebuje

Wersje to przypięcia z workflow wydania, w `.github/workflows/release.yml`:

- **Rust 1.97.1** przez rustup, z celem Kindle i Makefile wskazującym na niego:

  ```sh
  rustup toolchain install 1.97.1 --profile minimal --target armv7-unknown-linux-musleabihf
  export RUST_BIN="$(dirname "$(rustup which --toolchain 1.97.1 rustc)")"
  ```

  Bez `RUST_BIN` Makefile stawia `stable` rustup dla Apple Silicon pierwsze na PATH (zmienia się z
  każdym wydaniem Rusta i nie ma celu Kindle, chyba że go dodasz); na Macu Intel lub Linuksie ten
  folder nie istnieje, więc make uruchamia pierwsze `cargo` ze swojego PATH.
- **Zig 0.16.0** (ziglang.org) i **cargo-zigbuild 0.23.0**
  (`cargo install cargo-zigbuild --version 0.23.0 --locked`), dla builda Kindle. Nowszy zig przechodzi
  testy, ale tylko przypięty tworzy bajty wydania.
- Kompilator C (`clang`; gcc na Linuksie), make, curl i unzip: `build.rs` kompiluje dołączony C.
- **Python 3.12+** (tylko standardowa biblioteka), **Node.js 18+** i **LuaJIT**: testy je uruchamiają.
- Opcjonalne, każde pomijane z komunikatem, gdy brakuje: shellcheck, luacheck, dash i busybox
  (launchery i wtyczka KOReadera), `magick` z ImageMagick 7 (test obrazu studia Efektów) i `msgfmt` z
  GNU gettext (porównanie `.mo` języków; `make l10n` go potrzebuje). `make sim` potrzebuje SDL3 i
  pkg-config (na Macu `brew install sdl3 pkgconf`).

## Cele make

```sh
make check     # wszystko, co da się sprawdzić bez Kindle: testy jednostkowe, scenariusze
               # UI ze sprawdzaniem stanu, każdy język na każdym ekranie, serwer udostępniania,
               # Podgląd na żywo, Bluetooth, eksporter, build ARM, zip z launchermi i
               # wtyczką KOReadera, dwa czyste buildy porównane bajt po bajcie oraz pakiet KPM;
               # kończy się CHECK PASSED
make test      # testy jednostkowe (rust/tests), na hoście i w buildzie bezgłowym
make scenarios # tests/scenarios/*.qs na bezgłowym runnerze (SCENARIOS=... dla niektórych)
make kindle    # build/kindle/grimoire: statyczny ARMv7 hard-float (musl)
make dist      # kompletny instalowalny pakiet w dist/Grimoire-VERSION-COMMIT/
make package   # build/dist/Grimoire.zip
make kpm-package # build/kpm/*.kpkg i manifesty repozytorium preview/stable
make tools     # build/host/grimoire-export (używany przez tools/grimoire-pull)
make tablet    # build/host/scribe-tablet: pióro jako tablet Maca dla osu!
make l10n      # po/grimoire.pot ze źródeł, każdy po/<lang>.po zaktualizowany i skompilowany
make sim       # symulator drzewa C: okno SDL3, aplikacja przed portem na Rust
```

Na 15-rdzeniowym Macu `make -j15 check SCEN_JOBS=8` trwa około 7 minut, gdy cache buildów jest
ciepły; pierwszy build albo mniejsza lub bardziej zajęta maszyna trwa dłużej. `build/` rośnie do
kilku GB, a z czasem więcej, bo cargo trzyma starsze buildy w `build/cargo`. Dwa checkouty mogą to
uruchamiać jednocześnie na różnych portach (`GRIMOIRE_TEST_PORT`). Całe `make check` działa na macOS.
Na Linuksie działają testy jednostkowe i scenariusze (z dwiema znanymi awariami w
`docs/BACKLOG.md`); reszta nie była tam próbowana, a test udostępniania potrzebuje `sips` z macOS.

Używaj `make dist` dla buildów do zachowania lub instalacji. Zbiera instalacyjny ZIP, opcjonalny ZIP
poprawek KOReadera, pakiet KPM i manifesty, sumy kontrolne oraz rewizję źródeł w `dist/`. Foldery
wersji/commitów trzymają różne buildy osobno, a kolektor odmawia nadpisania różnych plików.
`DIST=/path/to/dist` wybiera wspólny folder wyjściowy między worktree. Pośrednie pliki buildów
zostają w `build/`. Zebranie builda go nie publikuje.

## Dokumentacja dla programistów

- `docs/DESIGN.md` opisuje architekturę, model odświeżania, model pędzla, języki i formaty plików
- `docs/FORMAT.md` pliki bajt po bajcie
- `docs/KINDLE.md` jak wpasowuje się w Kindle i KOReadera
- `docs/TESTING.md` testowanie na urządzeniu i układ CI
- `docs/SIM.md` symulator i bezgłowy runner scenariuszy
- `docs/TRANSLATING.md` jak ulepszyć tłumaczenie lub dodać język
- `docs/TABLET.md` narzędzie na Maca, które robi z pióra tablet przez SSH
