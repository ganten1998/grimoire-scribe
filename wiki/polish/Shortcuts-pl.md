[English](Shortcuts) | [日本語](Shortcuts-ja) | **Polski**

# Skróty klawiatury i kontrolera

Kontroler (albo klawiatura) to klawisze ekspresowe dla ręki, która nie trzyma pióra: strony,
narzędzia, cofnij i ponów, gumka i biały tusz, odnośniki i nazwy stron, w edytorze notatnika. Otwiera
i zamyka menu edytora, ale nie porusza się w nich (poza następne i poprzednie w wyborze narzędzi):
dotknij tam.

Włącz **Biblioteka > Ustawienia > Skróty > Włącz skróty**. To zapisuje początkowe mapowania do
`/mnt/us/grimoire/shortcuts.ini` tylko wtedy, gdy ten plik nie istnieje. Edytuj go przez USB/SSH i
wybierz **Wczytaj ponownie shortcuts.ini**; ten wiersz mówi, ile mapowań wczytano i ile wierszy
odrzucono. Arkusz listuje klawiatury i kontrolery oraz pokazuje ostatni kod klawisza lub przycisku
kontrolera. Wyłącz skróty, aby ponownie zamknąć opcjonalne urządzenia wejściowe.

Górne wiersze Ustawień to **Ekran**, **Czułość rysika**, **Skróty** i **Kalibruj pióro i dotyk**, w
tej kolejności; dolny wiersz **Paruj** otwiera część Bluetooth: Kontroler Bluetooth, Paruj
kontroler... i wiersz nazywający to, co podłączone.

## Układ klawiatury

Jeden układ dla klawiatury sparowanej z Kindle. Cmd klawiatury Apple zgłasza się jako Meta na Kindle,
a klawisz z wciśniętym Meta bez wiersza jest szukany ponownie z Ctrl w miejscu Meta, więc Cmd+Z to też
Cofnij. Skróty pozostają opcjonalne.

Klawisz może znaczyć jedno w każdym miejscu: **Biblioteka** (i ekran Udostępniania, gdzie działa
tylko Esc), **strona notatnika** (jakiekolwiek menu jest nad nią), **widok Stron**, **Graf** i
**Czułość rysika**.

| miejsce | klawisze |
|---|---|
| wszędzie | Esc `back`, Ctrl+S `save` |
| Biblioteka | Ctrl+N `new-notebook`, Ctrl+F `new-folder`, S `share`, O `settings`, Left/Right jej ekrany, G Graf |
| strona notatnika | 1 do 9 pióra, 0 i E gumka, W biały tusz, Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y cofnij i ponów, Left/Right i PageUp/PageDown strony, Home/End, [ i ] narzędzia, Shift+[ i Shift+] rozmiar pióra, Ctrl+N `new-page`, Ctrl+F `go-to-page`, Ctrl+L `add-link`, R `name-page`, Space `two-finger-up`, - i = powiększenie, Ctrl+0 cała strona, G L P T S, X przytrzymany gumuje |
| widok Stron | Left/Right jego ekrany, Ctrl+N, Ctrl+F, Ctrl+A `select-all`, Ctrl+Z i Ctrl+Shift+Z własne Cofnij i Ponów, P, G |
| Graf | strzałki `pan-*`, - i +, Ctrl+0, V `graph-view`, Ctrl+A, G |
| Czułość rysika | 1 do 9 karta tego pióra, 0 i E gumki |
| otwarta klawiatura | Left/Right kursor o literę, Home/End na początek i koniec (z Shiftem zaznaczają), Ctrl lub Cmd z A, C, X i V zaznacz wszystko, kopiuj, wytnij i wklej wpisany tekst; te przychodzą przed każdym skrótem i nie można ich przepiąć; nie w polu liczby |

**Esc** zjada jedną warstwę na naciśnięcie: pole lub pole liczby, okno dialogowe lub kartę,
przeciąganie, zaznaczenie, menu lub arkusz, umieszczany odnośnik, potem powrót z notatnika do
Biblioteki, powrót z Czułości rysika i Udostępniania, w górę z folderu i na górze Biblioteki
**Wyjść z Grimoire?**. **Space** robi to, co tam robi przesunięcie dwoma palcami w górę. **Ctrl+S**
zapisuje teraz stronę i jej notatnik. **R** otwiera klawiaturę nazywania strony.

## Profile urządzeń

Każda klawiatura lub kontroler, USB albo Bluetooth, może mieć własne skróty, zapisane i używane
ponownie, gdy jest podłączony. Ten bez własnych używa domyślnych: `shortcuts.ini` i `controller.ini`.

**Ustawienia > Skróty** pokazuje **Urządzenie**, **Własne skróty dla tego urządzenia**,
**Przyciski...**, **Wczytaj ponownie shortcuts.ini**, ostatnie wejście i Gotowe. Włącz **Własne
skróty dla tego urządzenia**: za pierwszym razem z kopii domyślnych, więc robi to, co robił, dopóki
nie zmienisz. Wyłączone, urządzenie znów używa domyślnych, a jego profil zostaje. Pliki mieszkają w
`/mnt/us/grimoire/shortcuts/`, nazwane slugiem urządzenia (jego nazwa małymi literami, ciągi innych
znaków jako `-`, potem numery producenta i produktu szesnastkowo: `8bitdo-pro-2-2dc8-6006`).
Wczytywane są do 32 profili.

## Przyciski kontrolera

**Ustawienia > Skróty > Przyciski...** pokazuje, co robi każdy przycisk, i uczy nowych. Jego lista ma
siedem grup: **Edytowanie**, **Narzędzia**, **Strony**, **Indeks**, **Menu**, **Ekran** i **System**.
Otwórz grupę, dotknij wiersza, a potem naciśnij przycisk; Gotowe wraca w górę. Jeden przycisk na
akcję: przycisk, który robił coś innego, robi teraz to (powiadomienie mówi, która akcja go straciła),
a inne przyciski akcji przestają ją robić. Wiersze poprzedni / następny i pierwszy / ostatni uczą się
dwóch naciśnięć, pierwszego, a potem drugiego. Dziesięć sekund bez naciśnięcia zostawia wiersz tak,
jak był. Przytrzymaj wiersz (0,6 s), aby zabrać jego przyciski; **Przywróć przyciski** usuwa
`controller.ini`.

Spusty kontrolera w układzie Xbox są czytane jako przyciski o nazwach LT i RT. Kontroler w trybie
Switch numeruje przyciski w kolejności Pro Controllera (B, A, Y, X, L, R, ZL, ZR, -, +, LS, RS, Home,
Capture); arkusz pokazuje te etykiety, a pliki zachowują numery.

`/mnt/us/grimoire/controller.ini` trzyma nauczone przyciski; dopóki istnieje, przyciski kontrolera
pochodzą tylko z niego, a wiersze `pad.` z `shortcuts.ini` są odsunięte. Wiersze klawiatury (`key.`)
zawsze pochodzą z `shortcuts.ini`.

## shortcuts.ini

Domyślna klawiatura mapuje 1–9 na dziewięć narzędzi rysunkowych w kolejności wyboru narzędzi (ołówek,
ołówek automatyczny, węgiel, długopis, fineliner, marker, pióro wieczne, pióro kaligraficzne,
zakreślacz), 0 i E na gumkę, W na biały tusz, Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y na cofnij i ponów,
left/right i PageUp/PageDown na strony, [ i ] na cykl narzędzi, a Shift+[ i Shift+] na rozmiar pióra.

Domyślne ustawienia gamepada używają konwencji Linuksa: South cofnij, East gumka, North ponów, West
biały, L1/R1 poprzednie/następne narzędzie, Select/Start poprzednia/następna strona, a D-pad
Left/Right strony i Up/Down narzędzia. To punkty startowe; **Przyciski...** służą do reszty.

```ini
key.1=tool:pencil
key.Ctrl+z=undo
key.Shift+Left=previous-page
key.code106=next-page
key.Ctrl+i=index
pad.East=tool:eraser
```

Starsze wersje odrzucają nowsze nazwy akcji (`save`, `back`, `two-finger-up`, `new-notebook`,
`new-folder`, `share`, `settings` i pozostałe z 2.0.0-beta.2), po jednym wierszu.
