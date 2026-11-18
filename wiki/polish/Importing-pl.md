[English](Importing) | [日本語](Importing-ja) | **Polski**

# Importowanie szablonów, zdjęć i PDF-ów

## Szablony

Skopiuj zdjęcia PNG lub JPG do `grimoire/templates/` przez USB, gdy Grimoire jest zamknięty
(pionowo, 3:4, 1860 x 2480 jest najlepsze; jasne linie na białym tle najlepiej wychodzą na e-ink).
W notatniku **Papier > Więcej układów...** je pokazuje; **Nowy notatnik** ma ten sam przycisk.

Szablon jest gotowy do wyboru, gdy jego kafelek pokaże zdjęcie (Grimoire najpierw sprawdza każdy nowy
plik w tle). Każdy notatnik zachowuje własną kopię używanego szablonu.

## Zdjęcia jako papier

Skopiuj zdjęcia JPG lub PNG do `grimoire/photos` przez USB. W notatniku otwórz **Papier** i wybierz
**Zdjęcia...**: zdjęcie staje się papierem strony, dopasowane do strony i obrócone pionowo tak, jak
zrobił je telefon, a ty piszesz po nim jak po każdej stronie (Cofnij przywraca stary papier). Dla
własnej strony najpierw dodaj nową stronę w Stronach.

## Strony, które już masz

Umieść folder ze zdjęciami stron (PNG lub JPG, jedno na stronę, na przykład PDF wyeksportowany jako
zdjęcia przez `pdftoppm -gray -png -r 200 file.pdf page`) w `grimoire/import/<Name>/` i uruchom
Grimoire: staje się notatnikiem `<Name>`, a folder przenosi się do `grimoire/import/done/`.

## PDF-y

Skopiuj PDF (planer, książkę) do `grimoire/import/` przez USB albo przez SSH, gdy Grimoire działa, i
otwórz Bibliotekę: powiadomienie na dole mówi, jak daleko jest import („Importowanie: strona 37 z
412”), a notatnik się pojawia. Jego strony to strony PDF-u, w kolorze tam, gdzie PDF ma kolor
(Scribe pokazuje je na szaro). PDF włożony do folderu w tym miejscu trafia do folderu Biblioteki o
tej nazwie, a PDF przenosi się do `import/done/`.

Do 2000 stron i 512 MB; dłuższy lub większy PDF albo PDF z hasłem jest odrzucany z komunikatem, a ten
sam PDF po raz drugi mówi „PDF już zaimportowany.”. Strona, której nie da się odczytać, staje się
pustą stroną, a koniec o tym mówi („Nie udało się odczytać niektórych stron.”); PDF bez żadnej
czytelnej strony nie jest importowany. PDF, który zawiedzie, czeka, aż go wymienisz (trzy próby);
ten, który zastał pełną pamięć, rusza, gdy zrobi się miejsce.

**Otwórz tę książkę** w KOReaderze przekazuje swój PDF do tego samego importu (zobacz
[KOReader](KOReader-pl)) i nadal sam rysuje DjVu oraz komiksy.

## Plik w dowolnym miejscu na Kindle

**Biblioteka > Nowy > Z pliku...** otwiera Pliki z tytułem „Importuj PDF lub zdjęcia”:

- **Importuj** (`grimoire/import`, z informacją, co robi każdy plik: Oczekuje, „Importowanie:
  strona 40 z 365”, Zaimportowano oraz wiersz Sprawdź ponownie)
- **Dokumenty**
- **Pobrane**
- **Cała pamięć** (notatniki, PDF-y i kosz Grimoire nigdy nie są listowane)

Dotknięcie folderu go otwiera; dotknięcie PDF-u otwiera jego kartę („14,2 MB, 30 paź” oraz Importuj
jako notatnik albo Otwórz jego notatnik, gdy notatnik już zawiera tę książkę; „ma notatki” pokazuje
się w jego wierszu), a drugie dotknięcie go importuje.

Notatnik zrobiony z PDF-u pamięta swój plik, więc karta takiego PDF-u oferuje Importuj jako notatnik
obok Otwórz jego notatnik: dokładnie ten sam plik mówi „PDF już zaimportowany.”, a inny plik, który
tylko wygląda jak ta książka według klucza KOReadera (na przykład edytowana kopia), staje się własnym
notatnikiem. Folder zdjęć importuje się z jego karty `...` (albo przytrzymania palca). Notatnik
bierze nazwę pliku, chyba że najpierw wpisałeś ją w arkuszu Nowy notatnik, i trafia do folderu z tego
arkusza.

Plik zostaje dokładnie tam, gdzie był: Grimoire tylko czyta poza własnymi folderami i nigdy nie
podąża za odnośnikiem (plik wybrany w samym imporcie trafia do `import/done/` jak ten wrzucony tam
bezpośrednio). Plik wybrany, gdy import z wcześniejszego uruchomienia jeszcze trwa, jest importowany
po jego zakończeniu.

## Zgumowany tusz znika z plików

Gdy notatnik się zamyka, całkowicie zgumowany tusz jest usuwany z plików jego stron w tle.
Pociągnięcie zgumowane tylko częściowo pozostaje w pliku w całości.
