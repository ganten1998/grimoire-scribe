[English](Fonts-and-Language) | [日本語](Fonts-and-Language-ja) | **Polski**

# Czcionki i język

## Język

Grimoire mówi po angielsku, hiszpańsku, japońsku i polsku. **Ustawienia > Język** oferuje
**Automatycznie**, **English**, **Español**, **日本語** i **Polski**. Automatycznie, domyślny
wybór, idzie za językiem ustawionym w Ustawieniach Kindle i daje angielski dla języka, którego
Grimoire jeszcze nie zna; jego wiersz nazywa język, który daje („Automatycznie (English)”). Jeśli
więc twój Kindle jest ustawiony na hiszpański, japoński lub polski, Grimoire jest w tym języku od
pierwszego uruchomienia po aktualizacji; **Ajustes > Idioma > English** (Ustawienia > Język po
hiszpańsku), **設定 > 言語 > English** (po japońsku) albo **Ustawienia > Język > English** (po
polsku) przełącza go z powrotem. Wybór działa od razu i zostaje zapamiętany.

Cały Grimoire idzie za nim: każdy ekran, strony udostępniania i studio Efektów, a także nazwa nowego
notatnika („Notatnik 20 paź”, „Cuaderno 20 oct”, „ノート 10月20日”). Twoje strony nigdy nie zmieniają
się z językiem: papier rysuje się wszędzie tak samo (nagłówki Cornell pozostają angielskie), a nazwy
nadane notatnikom i folderom pozostają takie, jak wpisałeś. Wtyczka KOReadera idzie za własnym
językiem KOReadera.

Poprawki i nowe języki są mile widziane; zobacz [Tłumaczenie](Translating-pl). Tłumaczenia są na
licencji Grimoire, GPL-3.0-or-later.

## Czcionki (Classic)

Kategoria Ekran w Ustawieniach ma w stylu Classic wiersz **Czcionka**, nazywający używaną czcionkę.
Otwiera arkusz Czcionki:

- **Terminess** (domyślna)
- **Atkinson Hyperlegible Next** (dla czytelników ze słabym wzrokiem: I, l i 1, O i 0 nie można
  pomylić)
- **Noto Sans**
- **Literata** (szeryfowa, stworzona do czytania na ekranach)

Każda jest nazwana sama sobą, z próbką używanej na górze. Dotknięcie czcionki stosuje ją na wszystkich
ekranach naraz i zapisuje; strzałka wstecz w arkuszu wraca do Ustawień, a X zamyka Ustawienia, obie
zachowują dotkniętą czcionkę (aby wrócić, dotknij czcionki, którą miałeś).

Twoje strony, etykiety papieru, zegar, PDF-y i zdjęcia nigdy nie zmieniają się z czcionką, podobnie
jak ikony; strony udostępniania w przeglądarce używają tej samej czcionki (już otwarta strona dostaje
ją po ponownym wczytaniu). Gothic zachowuje własny pikselowy krój, więc wiersza Czcionka tam nie ma.
Greckie, cyrylickie i inne litery, których czcionka nie ma, pochodzą z Terminess, a japońskie z
DotGothic16, jak w każdym stylu.

## Własne czcionki (Classic)

Skopiuj pliki `.ttf` lub `.otf` do `grimoire/fonts` przez USB (rodzina rozpakowana do własnego
folderu jest w porządku, jeden folder w głąb; do 200 plików, po 24 MB), a następnie otwórz
**Ustawienia > Czcionka**. Pod **Twoje czcionki** każdy plik pokazuje swoją nazwę narysowaną sobą,
gdy Grimoire sprawdzi go w tle („Sprawdzanie 3 czcionek...” pozostaje tymczasem w arkuszu, a plik
wciąż kopiowany czeka, aż będzie cały), albo jego wiersz jest wyszarzony i mówi dlaczego nie:

- „Nieczytelna czcionka.” (to nie czcionka, uszkodzona albo taka, której litery nic nie rysują)
- „Za duży (ponad 24 MB).”
- „Brak liter łacińskich.” (czcionka symboliczna)
- „Użyj .ttf lub .otf.” (czcionka internetowa)
- „Odczyt trwał za długo.”

Dotknij czcionki, aby jej użyć: stosuje się od razu i zostaje zapisana. Pogrubienie pochodzi z własnej
osi wagi czcionki, jeśli ją ma, inaczej z pliku Bold tej samej rodziny obok, a jeśli go nie ma,
Grimoire pogrubia zwykły krój. Litera, której twoja czcionka nie ma, pochodzi z Terminess (jej wiersz
mówi „Niektóre litery pochodzą z Terminess.”, gdy twój język ich potrzebuje), a kana i kanji z
DotGothic16, chyba że twoja czcionka je ma. Tekst w twojej czcionce może być trochę mniejszy lub
większy niż w Terminess: Grimoire dobiera rozmiar tak, aby wielkie litery były mniej więcej tej samej
wysokości, a słowa nie szersze, a etykieta, która nadal się nie mieści, kończy się „...”.

### Czyje czcionki

Twoja czcionka rysuje tylko własne słowa Grimoire (menu, nazwy, klawiaturę). Strony, etykiety
papieru, zegar, ikony, PDF-y i zdjęcia nigdy jej nie używają, podobnie jak strony udostępniania
(przeglądarka dostaje Terminess): Grimoire nigdy nie kopiuje twojej czcionki do strony, PDF-u ani
nigdzie indziej i nigdy jej nigdzie nie wysyła. Zostaje na tym Kindle. Używaj czcionek, do których
masz prawo.

### Powrót do Terminess

Dotknij Terminess w arkuszu Czcionki (gdy używana jest twoja czcionka, słowa samego arkusza są w
Terminess, więc zawsze można je odczytać; dotknięcie Terminess zabiera też twoją czcionkę po starcie
w Terminess spowodowanym awarią, aby następny start nie próbował jej znowu) albo usuń plik czcionki
przez USB: Grimoire jest w Terminess od następnego startu (działający Grimoire zachowuje wczytaną
kopię), a wiersz mówi „Brak pliku.”, dopóki nie wybierzesz innej czcionki albo nie włożysz pliku z
powrotem, co przywraca czcionkę.

Jeśli czcionka kiedykolwiek zatrzyma Grimoire przed narysowaniem pierwszego ekranu, następny start
jest sam w Terminess, start po nim próbuje czcionki jeszcze raz, a po drugiej awarii Grimoire
zostawia ją w spokoju, dopóki plik się nie zmieni.
