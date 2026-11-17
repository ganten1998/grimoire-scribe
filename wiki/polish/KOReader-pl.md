[English](KOReader) | [日本語](KOReader-ja) | **Polski**

# KOReader

Tak: KOReader może otworzyć Grimoire i wrócić z tobą do książki.

## Instalowanie wtyczki

Gdy KOReader jest zainstalowany (folder `koreader`, `extensions/koreader` lub `documents/koreader`
na Kindle), otwórz **Grimoire > Ustawienia > KOReader > Zainstaluj / zaktualizuj wtyczkę Grimoire**,
a następnie uruchom KOReader ponownie. To dodaje **Tools > Grimoire** do jego menu. Świeża instalacja
Grimoire zostawia KOReader w spokoju.

Otwórz Grimoire jest też akcją gestów KOReadera (**Ustawienia > Dotknięcia i gesty**). Menu mówi w
języku KOReadera, angielskim, hiszpańskim, japońskim lub polskim (**Abrir Grimoire**, **PDF de los
cuadernos**, **Restablecer la calibración del lápiz**; **Grimoireを開く**, **ノートのPDF**,
**ペンのキャリブレーションをリセット**; **Otwórz Grimoire**, **PDF-y notatników**, **Przywróć
kalibrację rysika**). Tylko uruchomienie z Biblioteki aktualizuje wtyczkę, więc po aktualizacji
Grimoire dotknij go raz w Bibliotece.

To wymaga KOReadera uruchomionego z jego elementu w Bibliotece albo z KUAL (start `koreader.sh`
KOReadera). Launcher KOReadera, który zostawia interfejs Kindle działający, też jest obsługiwany:
Grimoire pauzuje go na własną sesję i przywraca przed powrotem.

## Otwieranie i wyjście

- **Otwórz Grimoire**: KOReader zamyka się, a Grimoire otwiera się w jego miejsce, bez ekranu
  głównego Kindle pomiędzy. W Grimoire **Biblioteka > Wyjdź** brzmi **Powrót do KOReadera**; to
  albo przytrzymanie przycisku zasilania 4 sekundy przywraca KOReader z książką, którą miałeś
  otwartą (albo jego menedżer plików).
- **PDF-y notatników**: menedżer plików KOReadera w `grimoire/PDFs`.
- **Przywróć kalibrację rysika**: ta sama pomoc ratunkowa co element resetowania w Bibliotece.

Otworzony z KOReadera Grimoire nie może pokazać własnego wygaszacza Kindle jako ekranu uśpienia
(potrzebuje ekranów Kindle, które KOReader trzyma wstrzymane), więc pokazuje twoją stronę.

**Ekran > Ekran uśpienia: tapeta KOReadera** używa migawki skonfigurowanego przez KOReader własnego
obrazu, losowego obrazu lub okładki książki, eksportowanej za każdym razem, gdy **Tools > Grimoire**
przekazuje kontrolę. Działa też przy późniejszym starcie z Biblioteki. Jeśli żaden własny obraz nie
jest dostępny, pokazuje się twoja strona. Losowy folder wybiera jeden obraz na przekazanie; widżety
komunikatów i statusu KOReadera nie są kopiowane. Przy ustawionym kodzie dostępu urządzenia najpierw
pojawia się klawiatura kodu.

*Wykonaj skrypt powłoki* na `Grimoire.sh` w menedżerze plików KOReadera nadal odmawia, ponieważ
KOReader trzyma rysik i ekran dotykowy dla siebie (KOReader zgłasza status 3): użyj
**Tools > Grimoire**.

## Dodaj adnotację do tej strony

W książce w KOReaderze **Tools > Grimoire > Dodaj adnotację do tej strony w Grimoire** (także akcja
gestu) otwiera czytaną stronę w Grimoire, gotową do pisania. Grimoire prowadzi jeden notatnik na
książkę, nazwany jej tytułem (z pominięciem znaków, których nie umie pokazać), ze stronami w
kolejności książki; ponowne dodanie notatek do tej samej strony otwiera ją z tym, co napisałeś.
**Biblioteka > Powrót do KOReadera** wraca do tej samej książki i strony.

Strona PDF-u to cała strona; strona EPUB-a to ekran tak, jak układa go KOReader, więc po zmianie
rozmiaru czcionki strona może wejść ponownie jako nowa. Własny PDF notatnika z `grimoire/PDFs` jest
odrzucany, ponieważ robienie w nim notatek zrobiłoby notatnik z notatnika: otwórz ten notatnik w
Grimoire.

## Otwórz tę książkę w Grimoire

W PDF-ie, DjVu lub komiksie **Tools > Grimoire > Otwórz tę książkę w Grimoire** (także akcja gestu)
wkłada każdą stronę książki do jej notatnika Grimoire naraz, a potem otwiera czytaną stronę. W
menedżerze plików KOReadera przytrzymaj taki plik i wybierz **Otwórz za pomocą... > Grimoire**, aby
zrobić to samo bez otwierania go.

Dla DjVu lub komiksu KOReader najpierw rysuje każdą stronę, około sekundy lub dwóch na stronę na
Scribe, z komunikatem, który dotknięcie zatrzymuje. PDF idzie prosto do Grimoire, który sam go czyta
jak przy imporcie PDF-u (w kolorze tam, gdzie PDF ma kolor; powiadomienie na dole liczy strony, a
Biblioteka pozostaje tymczasem używalna), a potem otwiera czytaną stronę.

Jeśli w międzyczasie otworzyłeś coś innego, nic się samo nie otworzy: powiadomienie mówi, że import
się skończył, notatnik książki jest kartą w Bibliotece i otwiera się na tej stronie, gdy go dotkniesz.
Strony, które notatnik już ma, z tym, co na nich napisałeś, zostają bez zmian, więc ponowne otwarcie
tej samej książki dodaje tylko brakujące. To ten sam notatnik, którego używa **Dodaj adnotację do tej
strony**. Odnośniki samego PDF-u idą razem (zakładki planera). PDF z hasłem nadal idzie starszą
drogą, rysowany przez KOReader, podobnie PDF, którego Grimoire nie czyta (ukryty folder, własne
foldery KOReadera) lub większy niż 512 MB.

## Odnośniki dokumentu

Zakładki planera PDF są częścią jego stron. Dotknięcie podąża za jedną (odnośnik narysowany przez
ciebie ma pierwszeństwo tam, gdzie się nakładają); przytrzymanie palca na jednej rozpoczyna
Zaznaczanie, jak wszędzie na stronie. Wyczyść stronę je zachowuje, lasso wokół tego, co napisałeś w
komórce dnia, nigdy nie zabiera odnośnika komórki, a Kopiuj stronę, Przenieś do... i Kopiuj do
notatnika... przenoszą je (na strony, które też idą). Pasek Odnośników pokazuje je jako jeden wiersz
„Z PDF-a (N)”, który otwiera je pogrupowane według strony docelowej; Obszary rysuje ich ramki
kropkowane. Są w osobnym pliku (`doclinks.ini`), więc starszy Grimoire pokazuje strony i twoje
odnośniki bez nich.

## Zdjęcia

Skopiuj zdjęcia JPG lub PNG do `grimoire/photos` przez USB. W notatniku otwórz Papier i wybierz
**Zdjęcia...**: zdjęcie staje się papierem strony, dopasowane do strony i obrócone pionowo tak, jak
zrobił je telefon, a ty piszesz po nim jak po każdej stronie (Cofnij przywraca stary papier). Dla
własnej strony najpierw dodaj nową stronę w Stronach.

## Opcjonalne poprawki

**Biblioteka > Ustawienia > KOReader** instaluje lub aktualizuje wtyczkę, usuwa ją albo przełącza
każdą z trzech opcjonalnych poprawek: wyłączenie ukośnych gestów zrzutu ekranu, kwadratowe rogi
folderów i czcionki menu Terminess. Poprawka czcionek zawiera czcionkę. Uruchom KOReader ponownie po
zmianie poprawki lub wtyczki. Inna poprawka o tej samej nazwie pliku jest zachowywana; żadne poprawki
nie są stosowane samo przez aktualizację Grimoire.

Wtyczka jest opcjonalna. Aby ją usunąć, usuń `koreader/plugins/grimoire.koplugin`; dopóki KOReader
pozostaje zainstalowany, Grimoire jej nie przywraca. Aby ją przywrócić, także w KOReaderze
zainstalowanym od tego czasu lub tam, gdzie Biblioteka nie może uruchamiać skrypletów, skopiuj folder
`KOReader plugin (optional)/grimoire.koplugin` z zipa do `koreader/plugins` na Kindle (z
`Grimoire.sh` w `documents`, jak zwykle). Nigdy nie kopiuj folderu o nazwie `koreader` na Kindle:
mógłby zastąpić twojego KOReadera.
