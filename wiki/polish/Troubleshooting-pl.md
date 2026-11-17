[English](../english/Troubleshooting.md) | [日本語](../japanese/Troubleshooting-ja.md) | **Polski**

# Rozwiązywanie problemów

## Znane problemy

- **Pierwszy ekran KOReadera może być niekompletny na firmware 5.20.1.** KOReader może narysować
  swój pierwszy ekran z brakującymi częściami, także gdy wraca z Grimoire. To problem KOReadera
  ([koreader issue 16283](https://github.com/koreader/koreader/issues/16283)), którego Grimoire nie
  naprawia.

## Szybkie poprawki

- **Utknąłeś?** Przytrzymaj przycisk zasilania 4 sekundy. Grimoire zapisuje stronę i zamyka się, a
  Kindle wraca. Jeśli nawet to nic nie robi, przytrzymaj go, aż Kindle się zrestartuje (około 40
  sekund); restart zachowuje notatniki. Nigdy nie używaj kodu resetowania kodu dostępu 111222777,
  aby się z czegokolwiek wydostać: kasuje Kindle razem z notatnikami.
- **Kod dostępu Kindle jest wyłączony po restarcie**: Kindle zrestartował się, gdy Grimoire był
  otwarty (`grimoire.log` tak mówi). Włącz kod dostępu ponownie w Ustawieniach Kindle (Opcje
  urządzenia > Kod dostępu urządzenia); Grimoire zostawia to tobie.
- **„Ta strona pochodzi z nowszej wersji i będzie tylko do odczytu.”**: zaktualizuj Grimoire, aby
  zmienić tę stronę. Nic na niej nie przepada (zobacz Przenoszenie między wersjami).
- **KOReader**: otwórz Grimoire z KOReadera przez **Tools > Grimoire**, nigdy przez *Wykonaj skrypt
  powłoki* (zobacz [KOReader](KOReader-pl)). Jeśli po dotknięciu Grimoire w Bibliotece u góry ekranu
  pojawi się „Grimoire nie uruchomił się: KOReader działa.”, KOReader nadal działa w tle (na przykład
  po awarii): zrestartuj Kindle.
- **KOReader mówi „Grimoire nie mógł się uruchomić. Zobacz grimoire/grimoire.log.”**: przekazanie nie
  uruchomiło Grimoire. Wiersz `grimoire.sh: not starting` w `grimoire/grimoire.log` mówi dlaczego (na
  przykład KOReader nie puścił pióra i ekranu dotykowego), a `already running` znaczy, że inny
  Grimoire był jeszcze otwarty. Gdy nie ma żadnego, przekazanie zawiodło w KOReaderze: własny log
  KOReadera, `koreader/crash.log`, ma o tym wiersz zaczynający się `Grimoire:`. „Ten Grimoire nie
  może otworzyć się z KOReadera” znaczy, że `Grimoire.sh` w `documents` jest starszy niż 1.1:
  skopiuj nowszy.
- **Pociągnięcia lądują obok końcówki pióra**: **Ustawienia > Kalibruj pióro i dotyk**. Jeśli nie
  możesz tam dotrzeć, dotknij **Grimoire - reset pen calibration** w Bibliotece.
- **Wszystko jest do góry nogami** (możliwe na Scribe z 2022): umieść pusty plik o nazwie
  `screen-flip` lub `screen-noflip` w folderze `grimoire` i uruchom ponownie.
- **Przez ekran przebijają fragmenty własnego ekranu Kindle**: Grimoire zamalowuje je sam. Jeśli to
  się powtarza, umieść pusty plik o nazwie `stop-framework` w folderze `grimoire`; Grimoire wtedy
  całkowicie zatrzymuje interfejs Kindle na czas działania (powrót po Wyjściu trwa kilka sekund
  dłużej). Otworzony z KOReadera plik nic nie robi: KOReader sam pauzuje interfejs Kindle.
- **Nic się nie dzieje, gdy dotykasz Grimoire**: zajrzyj do `grimoire/grimoire.log`. Dołącz go do
  każdego zgłoszenia błędu.

## Przenoszenie między wersjami

Aktualizacja to jeden plik, podobnie powrót. Od 1.1.0-beta.1 Grimoire zachowuje to, co zapisze nowsza
wersja i czego nie rozumie (dane tuszu, nowsze papiery, dodatkowe dane stron, nieznane wiersze w
`settings.ini` i `notebook.ini`) i zapisuje to z powrotem bez zmian:

- Strona, której nie może w pełni pokazać, otwiera się **tylko do odczytu**, z „Ta strona pochodzi z
  nowszej wersji i będzie tylko do odczytu.”. Pisanie, gumowanie,
  czyszczenie i zmiana papieru są na niej odrzucane; usuwanie nadal działa. Zaktualizuj Grimoire,
  aby ją edytować.
- Papier z nowszej wersji pokazuje się jako czysty papier o nazwie **Nowszy papier** i pozostaje
  papierem strony. Nowe strony dodane po takiej stronie też go dostają.
- Ustawienie z nowszej wersji, którego ta nie ma, działa tutaj jako najbliższy dostępny wybór
  (przełącznik z trzecim stanem liczy się jako włączony lub wyłączony) i zostaje w pliku, dopóki nie
  zmienisz tego ustawienia.
- Nazwa notatnika lub folderu dłuższa niż 95 bajtów (około 47 liter z akcentami) jest przycinana,
  nigdy w środku litery z akcentem.

1.0.0-beta i 1.0.0-rc.1 tego nie potrafią. Otwórz tam notatnik po zapisie nowszej wersji, a dane mogą
przepaść: odkładają stronę w nowszym formacie jako `p<id>.qpg.damaged-<time>` i pokazują ją jako
pustą (samo spojrzenie na nią w Stronach to robi), a przy zapisie porzucają nowsze dane.
1.1.0-beta.1 i późniejsze przywracają takie strony, gdy ponownie otworzysz notatnik, i zachowują obie
kopie, jeśli strona została w międzyczasie zapisana ponownie; strona usunięta w starszej wersji
pozostaje odłożona.

**Scribe'y dwóch rozmiarów.** Scribe z 2022 i 2024 mają ekran 1860x2480, Scribe 3 i Scribe Colorsoft
większy, 1986x2648. Od 1.1 notatnik zachowuje rozmiar strony, z którym powstał: na Scribe o innym
rozmiarze jego strony są pokazywane dopasowane do ekranu, pismo na swoich liniach, a jego nowe strony,
PDF-y i PNG mają rozmiar notatnika. Notatnik sprzed 1.1 bierze rozmiar, który ma większość jego stron.
Kalibracja pióra i dotyku idzie za folderem `grimoire` skopiowanym na Scribe o innym rozmiarze:
Grimoire skaluje ją do nowego ekranu przy starcie. Wersje przed 1.1 rozciągają każdą stronę do
własnego ekranu (o 7% między rozmiarami, co zsuwa pismo z linii); z powrotem w 1.1 taka strona wraca
do rozmiaru swojego notatnika. Więc powrót do starszej wersji na Scribe o innym rozmiarze przesuwa
tusz na każdej stronie, którą tam edytuje.

**Język.** 1.0.0-beta i 1.0.0-rc.1 zapominają wybór Ustawienia > Język przy następnym zapisie
ustawień, więc po powrocie do 1.1 lub późniejszej język znów jest Automatycznie.

**KOReader.** Starsza wersja zostawia wtyczkę Grimoire KOReadera tam, gdzie jest: Tools > Grimoire
zostaje w KOReaderze, PDF-y notatników i Przywróć kalibrację rysika nadal działają, a Otwórz Grimoire
mówi „Ten Grimoire nie może otworzyć się z KOReadera”, ponieważ `Grimoire.sh` starszy niż 1.1 nie może
być otwarty z KOReadera. Po powrocie do 1.1 Otwórz Grimoire znów działa.

**Zdjęcia jako papier.** W 1.0.0-beta strona na szablonie, zaimportowana strona lub strona z
notatkami pokazuje czysty papier ze swoim tuszem. Po powrocie do 1.1 te strony odzyskują zdjęcia, gdy
notatnik się otwiera. Starsze wersje pokazują też ponownie zgumowany tusz, bo nie znają białego tuszu
ani dokładnej gumki, i porzucają przypięte pióra z `settings.ini`.
