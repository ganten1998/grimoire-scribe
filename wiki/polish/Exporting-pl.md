[English](Exporting) | [日本語](Exporting-ja) | **Polski**

# Przenoszenie stron z Kindla

Trzy drogi. Wszystkie używają tego samego renderera, więc strona wygląda identycznie na Kindle,
w PNG i w PDF.

## Wi-Fi: nic nie trzeba instalować

Na Kindle: włącz Wi-Fi (**Ustawienia > Wi-Fi** wybiera też sieć), a potem **Biblioteka > Udostępnij**.
Pokazuje kod QR i pod nim, dużym drukiem, ten sam adres, na przykład
`http://192.168.1.23:8765`. Skieruj kamerę telefonu na kod albo otwórz adres w przeglądarce na
dowolnym komputerze lub telefonie w tej samej sieci.

- każdy folder i notatnik z miniaturą
- dla notatnika: **PDF**, **ZIP** (PDF + jeden PNG na stronę + pliki źródłowe), **Efekty** (niżej) i
  czytnik z jedną stroną naraz, licznikiem stron i bezpośrednim wpisywaniem numeru u góry
- dla folderu: zip; u góry: **Pobierz wszystko**

Trzymaj ekran Udostępniania otwarty podczas pobierania; jego opuszczenie zatrzymuje serwer. Gdy
Kindle zasypia na ekranie Udostępniania, serwer nie odpowiada na nic nowego: żądana wtedy strona
czeka i wczyta się po wybudzeniu, jeśli przeglądarka się nie poddała.

Strony są w języku Grimoire. Nazwy pobieranych plików i punkty końcowe do skryptów są takie same w
każdym języku. Pliki źródłowe w zipie to `notebook.ini` notatnika, jego `source.ini`, gdy robi
notatki w książce, jego strony `p<id>.qpg` i jego zdjęcia (`bg/`).

Każdy eksport ma rozmiar strony notatnika (1860x2480 lub 1986x2648), niezależnie od tego, który
Kindle go tworzy.

### Efekty

**Efekty** (na każdym notatniku i pod każdą stroną) otwiera studio, które tworzy nowe obrazy twoich
stron. Kindle wysyła każdą stronę jako dwie warstwy, pusty papier i sam tusz, a przeglądarka składa
je z powrotem po twojemu:

- **Tło**: białe, przezroczyste, kolor lub gradient
- **Linie papieru**: pokazane lub ukryte, w dowolnym kolorze i sile
- **Tusz**: jak napisany, jeden kolor lub gradient przez pismo
- **Wygląd**: ton, ciepło, odcień, jasność, kontrast
- **Kadrowanie i rozmiar**: cała strona albo przycięta do tuszu z marginesem; 100%, 50% lub 25%
- **Presety**: Przezroczysty, Podpis, Ciepły, Sepia, Pergamin, Blueprint, Tablica, Noc, Tusz
  zachodu słońca, Zorza

Pobierz oglądaną stronę jako PNG albo każdą stronę jako zip PNG-ów lub jeden PDF. Przezroczyste PNG
zachowują miękkie krawędzie tuszu jako częściową przezroczystość. Przeglądarka pamięta ostatnie
ustawienia. Każde sterowanie to jedna operacja ImageMagick, a studio wypisuje polecenie `magick`,
które tworzy ten sam obraz.

Bezpieczeństwo: serwer nie ma hasła. Nasłuchuje tylko, gdy ekran Udostępniania jest otwarty, otwór w
zaporze jest tylko na interfejsie Wi-Fi, jest tylko do odczytu (GET) i odrzuca żądania, których
`Host` nie jest adresem IP. Nie otwieraj Udostępniania w sieci, której nie ufasz.

## Kabel USB

Przy włączonym **Ustawienia > Kopiuj PDF-y do grimoire/PDFs** (domyślnie) zamknięcie notatnika
zapisuje `grimoire/PDFs/<folder>/<name>.pdf` w tle. Zmiana nazwy, przeniesienie lub usunięcie
notatnika przenosi lub usuwa też jego PDF. Podłącz Kindle i skopiuj je.

Pliki źródłowe notatników są w folderze `grimoire`, jeśli chcesz wszystko. PDF-y są trzymane poza
`documents/`, aby Kindle ich nie indeksował.

## Z terminala

```sh
tools/grimoire-pull --http 192.168.1.23     # przez Wi-Fi (ekran Udostępniania otwarty), bez SSH
tools/grimoire-pull                         # przez SSH: rsync + renderowanie tutaj
tools/grimoire-pull --png --open            # także strony PNG; otwórz folder po zakończeniu
```

Wynik: `~/Grimoire/<folder>/<notebook>.pdf` (oraz `<notebook>/page-NNN.png`); `$GRIMOIRE_OUT` to
zmienia.

Tryb SSH rsyncuje `/mnt/us/grimoire/notebooks/` z `$GRIMOIRE_HOST` (domyślnie `kindle`) do
`~/Library/Caches/grimoire`, a potem uruchamia `build/host/grimoire-export` (`make tools`). Jest
przyrostowy: tylko notatniki zmienione od ostatniego eksportu są renderowane ponownie.

`grimoire-export [--png] [--zip] [--force] <grimoire-root> <out-dir>` działa też na dowolnej kopii
folderu `grimoire`, na przykład skopiowanej przez USB. Zbuduj go z tego samego wydania co Grimoire
na Kindle (`make tools` ponownie po aktualizacji).

## KOReader

KOReader otwiera PDF-y z `grimoire/PDFs` ze swojego menedżera plików albo przez
**Tools > Grimoire > Notebook PDFs**, gdy wtyczka jest zainstalowana. Grimoire zapisuje PDF notatnika
ponownie, ilekroć notatnik zamknie się po zmianie, i zmienia jego nazwę, przenosi go lub usuwa razem
z notatnikiem, więc zakreślenia lub notatki, które KOReader zapisze w tym pliku, przepadają: czytaj
te pliki, a notatki rób na kopiach.
