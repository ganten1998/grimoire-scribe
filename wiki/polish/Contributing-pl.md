[English](../english/Contributing.md) | [日本語](../japanese/Contributing-ja.md) | **Polski**

# Współtworzenie

Dziękujemy za rozważenie wkładu w Grimoire! To projekt otwartoźródłowy na licencji GPL-3.0; wkład
każdego rodzaju jest mile widziany.

## Sposoby współtworzenia

### Zgłaszanie błędów

Zgłoszenia są całkowicie otwarte: zgłoś błąd w dowolnym momencie, bez pozwolenia.

- Najpierw przeszukaj istniejące zgłoszenia, aby uniknąć duplikatów
- Używaj jasnych, opisowych tytułów
- Podaj wersję firmware Kindle, metodę jailbreaka i kroki do odtworzenia
- Dołącz `grimoire/grimoire.log`, jeśli jest dostępny

### Propozycje funkcji

Propozycje funkcji w zgłoszeniach też są całkowicie otwarte: po prostu otwórz zgłoszenie i opisz,
czego chcesz i dlaczego. Nie musisz pytać najpierw; jeśli to dobre dla projektu, omówimy to tam.

### Tłumaczenie

Grimoire jest dostępny po angielsku, hiszpańsku, japońsku i polsku. Zobacz
[Tłumaczenie](Translating-pl), aby ulepszyć tłumaczenie lub dodać język. Wszystkie tłumaczenia są na
licencji projektu GPL-3.0-or-later.

### Wkład w kod

- Możesz otwierać PR-y: wyślij go, gdy masz coś działającego
- Ale jeśli twój PR ma konflikt, **to twój problem, nie nasz**. Zrób rebase i rozwiąż go sam, zanim
  poprosisz o review; nie naprawimy za ciebie konfliktów scalania
- Kod jest głównie w Rust (`rust/` crate) z kilkoma skryptami powłoki i Pythonem
- Uruchom `make check`, aby sprawdzić, czy zmiany przechodzą automatyczne testy
- Zobacz [Makefile](https://github.com/ganten1998/grimoire-scribe/blob/main/Makefile), aby poznać
  dostępne polecenia (`make test`, `make scenarios`, `make l10n-check` itd.)
- Kod jest budowany z Rust 1.97.1, Zig 0.16.0 i `cargo-zigbuild 0.23.0` dla celu Kindle

### Dokumentacja

Dokumentacja jest w katalogu `docs/` i jest napisana w Markdown. Zobacz istniejące dokumenty, aby
poznać konwencje stylu. Zmiany w dokumentacji są sprawdzane w ramach testów projektu.

### Korzystanie z LLM

- Używanie LLM do pisania kodu jest w porządku: nie zabraniamy tego i nie poprosimy cię o deklarację
- Ale twój wynik jest twój: cokolwiek włożysz do PR-a, jest **twoje**, i twoja wina, jeśli zrobi coś
  złego
- „Model to napisał” nie jest wymówką. Odpowiadasz za przegląd, zrozumienie i przetestowanie każdej
  linii, którą wysyłasz
- Nie wrzucaj nieprzejrzanego wygenerowanego kodu do PR-a i nie oczekuj, że znajdziemy jego błędy za
  ciebie

## Wiki dla programistów

**Ten projekt ma sekcję wiki dla programistów, którzy chcą wnosić kod.** Wiki zawiera głębsze
przewodniki techniczne, dokumenty architektury i zaawansowane samouczki, które nie mieszczą się w
głównym przewodniku współtworzenia. Tematy obejmują: budowanie ze źródeł, strukturę crate'a Rust,
kompilację dla Kindle, runner scenariuszy, workflow lokalizacji i konfigurację CI/CD.

Zacznij od [Budowanie ze źródeł](Building-from-Source-pl), gdzie są zestawy narzędzi, cele make i
testy.

## Konfiguracja środowiska

1. **Zrób fork repozytorium** i sklonuj swój fork
2. **Zainstaluj wymagania**:
   - Rust 1.97.1 z celem Kindle:
     `rustup toolchain install 1.97.1 --profile minimal --target armv7-unknown-linux-musleabihf`
   - Zig 0.16.0 i `cargo-zigbuild 0.23.0`
   - Kompilator C (`clang`), make, curl i unzip
   - Python 3.12+, Node.js 18+ i LuaJIT
3. **Zbuduj i przetestuj**:
   - `make check` uruchamia pełny zestaw testów, w tym testy jednostkowe, scenariusze, testy
     lokalizacji i inne
   - `make test` uruchamia tylko testy jednostkowe Rust
   - `make scenarios` uruchamia skryptowe scenariusze UI
4. **Wprowadź zmianę** i upewnij się, że `make check` przechodzi
5. **Wyślij pull request** z jasnym opisem, co zmieniłeś i dlaczego

## Struktura projektu

- `rust/` - crate źródłowy Rust
- `docs/` - dokumentacja (w tym jak budować, testować i współtworzyć)
- `tests/scenarios/` - skryptowe scenariusze testów UI
- `po/` - pliki tłumaczeń
- `.github/workflows/` - konfiguracja CI/CD

## Zasady

- Zachowuj kulturę w zgłoszeniach i PR-ach
- Maintainerzy decydują, co zostanie scalone; otwarty PR nie znaczy, że zostanie przyjęty
- Nie oczekuj, że naprawimy twoje gałęzie, konflikty lub niedziałające CI za ciebie
