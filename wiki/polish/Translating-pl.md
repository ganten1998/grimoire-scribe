[English](../english/Translating.md) | [日本語](../japanese/Translating-ja.md) | **Polski**

# Tłumaczenie Grimoire

Grimoire mówi po angielsku, hiszpańsku, japońsku i polsku. Ta strona mówi, jak ulepszyć tłumaczenie
i jak dodać język. Pełne zasady są w `docs/TRANSLATING.md` w repozytorium.

## Co jest dostępne

```
po/grimoire.pot      każdy ciąg do przetłumaczenia, tworzony ze źródeł przez make l10n
po/<lang>.po         tłumaczenia jednego języka (po/es.po), plik edytowany przez tłumaczy
po/<lang>.mo         ten plik skompilowany przez msgfmt: zatwierdzony i osadzony w binarce
po/es-glossary.md    słowa wybrane przez hiszpański, z rozważonymi alternatywami
po/ja-glossary.md    słowa wybrane przez japoński oraz notatki o czcionce i dacie
```

- Angielski jest w kodzie (`src/*.c` i `web/studio.js`). Ciąg, którego katalog nie ma lub który jest
  pusty, pokazuje się po angielsku.
- `po/grimoire.pot` zawiera każdy ciąg pokazywany przez ekrany i studio Efektów, każdy ze swoim
  komentarzem `TRANSLATORS` i plikiem, z którego pochodzi.
- Budowanie nie potrzebuje gettext: binarka osadza zatwierdzone pliki `.mo`. `make l10n`, które
  wymaga GNU gettext, tworzy ponownie `po/grimoire.pot` ze źródeł, scala go do każdego
  `po/<lang>.po` i kompiluje każdy do jego `.mo`. `make l10n-check`, część `make check`, sprawdza
  każdy język na każdym ekranie.
- Wtyczka Grimoire do KOReadera ma kilka własnych słów, których nie ma w `po/`: tablicę `STRINGS` w
  `kindle/koreader/grimoire.koplugin/main.lua` i opis w `_meta.lua`. Wybiera je według własnego
  ustawienia języka KOReadera.
- Tłumaczenia są na licencji Grimoire, GPL-3.0-or-later, jak mówi nagłówek `po/es.po`.

## Ulepszanie hiszpańskiego

Hiszpański to jeden neutralny `es` dla Hiszpanii i Ameryki Łacińskiej, zwracający się do czytelnika
jako *tú*. `po/es-glossary.md` listuje każdy wybór słów, z rozważonymi alternatywami: zacznij tam.

## Ulepszanie japońskiego

Japoński to jeden `ja`, zwracający się do czytelnika w uprzejmej formie です・ます w zdaniach i
etykietach rzeczownikowych dla przycisków. `po/ja-glossary.md` listuje główne wybory oraz konwencje
daty i liczników: zacznij tam. Jego kana i kanji pochodzą z DotGothic16, pikselowej czcionki
osadzonej jako japoński zapas (podzbiór do kana i częstych kanji), więc styl Gothic przetrwa.

## Ulepszanie polskiego

Polski to jeden `pl`, zwracający się do czytelnika w drugiej osobie liczby pojedynczej. Liczby mnogie
mają trzy formy: pierwsza dla 1, druga dla 2–4 (z wyjątkami dla 12–14), trzecia dla pozostałych.
Terminologia interfejsu jest w `po/pl.po`; nowe ciągi powinny jej odpowiadać zamiast wprowadzać nowe
warianty tych samych pojęć.

## Zasady tłumaczenia

- **Miejsce.** Większość ciągów siedzi w widżecie o stałej szerokości, a ich komentarz `TRANSLATORS`
  podaje miejsce w pikselach przy rozmiarze tekstu: „284 px at 34 px” to tekst rysowany na 34 px
  wysokości z 284 px szerokości. `make l10n-check` listuje każdy skrócony tekst. Słowo, które nadal
  się nie mieści, potrzebuje krótszego.
- **Dotknięcia górnego paska.** Słowo górnego paska szersze niż 153 px w Gothic (159 px dla
  wszystkich poza New) lub 175 px w Classic poszerza przyciski na tyle, że New odsuwa się od dotknięć
  scenariusza. Trzymaj te słowa tak wąskie.
- **Argumenty.** Zachowaj każdy `%s`, `%d` i podobne, tego samego rodzaju i tyle samo, ile ma
  angielski. Aby ustawić je w innej kolejności, ponumeruj wszystkie: `%2$d %1$s`. Nigdy nie mieszaj
  numerowanych i zwykłych, użyj każdego argumentu i zapisz znak procentu jako `%%` tam, gdzie robi to
  angielski.
- **Całe zdania.** Każdy ciąg to całe zdanie lub etykieta, z wartościami wchodzącymi przez `%s` i
  `%d`. Wiersz, przez który przełącza dotknięcie, to jedno zdanie na wartość: tłumacz każdy
  całościowo.
- **Liczby mnogie.** Ciąg z `msgid_plural` potrzebuje każdej formy, jaką ma język (`msgstr[0]`,
  `msgstr[1]`...), w kolejności reguły Plural-Forms.
- **Konteksty.** `msgctxt` rozróżnia dwa znaczenia tego samego angielskiego (miesiąc „May”, papier
  „Blank”): tłumacz każde w swoim znaczeniu. Konteksty to `paper`, `tool`, `music part`, `month`,
  `date`, `key`, `keyboard`, `share link`, `folder`, `sleep after` i `studio`.
- **Znaki.** Każdy znak musi być taki, który potrafi narysować krój: Jersey 10 i Jacquard 24
  (Gothic), Terminess regularny i pogrubiony (Classic) albo, dla japońskiego, DotGothic16. Czcionki
  Classic z Ustawienia > Czcionka też muszą mieć każdy znak poza japońskimi.
- **Nazwy czcionek** (Terminess, Atkinson Hyperlegible Next, Noto Sans, Literata) nigdy nie są
  tłumaczone.
- **Co zostaje bez zmian**: Grimoire, Kindle, Kindle Scribe, KOReader, Wi-Fi, PDF, PNG, ZIP,
  ImageMagick; nazwy plików i folderów na Kindle (`grimoire/PDFs`, `grimoire/trash`,
  `grimoire.log`); polecenie `tools/grimoire-pull --http`. Nagłówki drukowane na papierze Cornell
  należą do strony i nie są w katalogu.

## Dodawanie języka

Weźmy francuski, `fr`:

1. Zrób jego katalog z szablonu:

   ```
   msginit --input=po/grimoire.pot --locale=fr --output-file=po/fr.po --no-translator
   ```

   `msginit` wypełnia nagłówek, w tym Plural-Forms (dla francuskiego
   `nplurals=2; plural=(n > 1);`). Nadaj jego komentarzowi nagłówka wiersz licencji, który ma
   `po/es.po`.
2. Dodaj jego wiersz do `k_langs` w `src/i18n.c`, po pozostałych: kod, nazwę w nim samym
   („Français”), liczbę form liczby mnogiej i regułę liczby mnogiej, funkcję z liczby na indeks jej
   formy, napisaną zgodnie z Plural-Forms.
3. Dodaj jego wiersz Plural-Forms do `k_rules` w `tests/test_i18n.c`, tak jak ma go nagłówek
   katalogu, oraz sprawdzenia funkcji w teście `plurals`: `NG_` przy 0, 1 i 2 z formami, które daje
   katalog.
4. Przetłumacz, potem uruchom `make l10n`: scala szablon do `po/fr.po` i kompiluje `po/fr.mo`.
   `make l10n-check` sprawdza każdy ekran; `make test` uruchamia `tests/test_i18n.c`.

## Wypróbowanie zmiany bez budowania

Aby wypróbować zmianę w symulatorze desktopowym bez ponownego budowania, skompiluj katalog ręcznie i
wskaż go symulatorowi:

```sh
mkdir -p /tmp/l10n && msgfmt --check -o /tmp/l10n/es.mo po/es.po
GRIMOIRE_L10N_DIR=/tmp/l10n GRIMOIRE_LANG=es ./build/host/grimoire-sim
```

Jego log mówi wtedy `i18n: (simulator) the catalog for es comes from /tmp/l10n/es.mo`.

Bez gettext nadal możesz edytować `po/es.po`, `po/ja.po` lub `po/pl.po` i wysłać; kto buduje,
uruchomi `make l10n`. Zatwierdź `po/<lang>.po` i `po/<lang>.mo` razem, z `po/grimoire.pot`, jeśli
się zmienił: `make l10n-check` zawodzi, gdy `.mo` nie jest tym, co msgfmt robi z `.po`.
