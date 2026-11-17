[English](../english/Home.md) | [日本語](../japanese/Home-ja.md) | **Polski**

# Grimoire

**Notatnik do pisania odręcznego dla Kindla Scribe z jailbreakiem.** Papier w linie, w kropki,
w kratkę i w układzie Cornella, nuty oraz tabulaturę gitarową lub basową możesz dowolnie łączyć
i nakładać; do tego dziewięć piór dostrojonych do nacisku i pochylenia rysika Scribe, foldery
oraz trzy proste sposoby przeniesienia stron na komputer w formacie PDF lub PNG. Wszystko zostaje
na Kindle w otwartych plikach: bez konta, chmury i śledzenia.

Wygląda jak zamek z dawnej gry konsolowej: pikselowe tytuły w stylu blackletter, pikselowa
czcionka, ramy z rzeźbionego kamienia i ostrołuki (prosty styl **Classic** włączysz jednym
dotknięciem w Ustawieniach).

Wolne oprogramowanie na licencji GPL-3.0. Projekt niezależny: nie jest powiązany z Amazonem
ani przez Amazon firmowany.

## Czy zadziała na moim Kindle?

Potrzebujesz **Kindle Scribe** z jailbreakiem, który uruchamia skryplety (pliki `.sh` uruchamiane
dotknięciem w Bibliotece). Wszystkie obecne jailbreaki dla Scribe (Vera, SpiderCat, WinterBreak,
Sanctuary) to obsługują; jak wykonać jailbreak, przeczytasz na
[kindlemodding.org](https://kindlemodding.org). W przypadku starszego jailbreaka bez skrypletów
Grimoire możesz otworzyć przez KOReader (patrz tabela).

| Kindle | Stan |
|---|---|
| Kindle Scribe (2024, 2. generacja), firmware 5.17–5.20.1 | **Działa.** Używany codziennie na 5.20.1.0.2; działa też na 5.20.1 (Vera) |
| Kindle Scribe (2022, 1. generacja), firmware 5.16.3 lub nowszy | **Działa.** Używany codziennie na 5.20.1.0.2; działa też na 5.20.1 (Vera). Jeśli ekran jest do góry nogami, zobacz Rozwiązywanie problemów. |
| Kindle Scribe (2022) z firmwarem starszym niż 5.16.3 | Powinien działać, ale dla tych starszych jailbreaków nie ma jeszcze launchera w Bibliotece. Z KOReaderem powinien otwierać się z Tools > Grimoire po ręcznym skopiowaniu wtyczki (zobacz Czy działa w KOReaderze?). |
| Kindle Scribe 3 i Scribe Colorsoft (2025) | **Nietestowane.** Nie ma jeszcze publicznego jailbreaka. Grimoire zna już ich większy ekran (1986x2648): notatniki zachowują rozmiar strony między nimi a starszymi modelami Scribe (zobacz Przenoszenie między wersjami). Na kolorowym ekranie Colorsoft, po każdym szybkim odświeżeniu, Grimoire czeka, aż sterownik je odczyta; wyjątkiem jest tusz samego pióra (pisanie nigdy nie czeka), więc przyciski nie powinny wyglądać na prążkowane. |
| Paperwhite, Oasis, Basic i inne Kindle bez rysika | Nieprzydatny: bez rysika nie da się pisać. |

Najnowszy firmware to 5.20.1 (stan na październik 2026). Przed aktualizacją sprawdź, czy twój
jailbreak go przetrwa; większość osób blokuje aktualizacje OTA.

## Dokumentacja

| Strona | Czego dotyczy |
|---|---|
| [Instalacja](Install-pl) | Wymagania, archiwum ZIP wydania, KPM, aktualizacja i odinstalowanie |
| [Korzystanie z Grimoire](Using-Grimoire-pl) | Biblioteka, papier, pióra, pasek narzędzi, Indeks, odnośniki, Podgląd na żywo, Wi-Fi, uśpienie i kod dostępu |
| [Czcionki i język](Fonts-and-Language-pl) | Angielski, hiszpański, japoński i polski, czcionki Classic oraz własne czcionki |
| [Importowanie](Importing-pl) | Szablony, zdjęcia, PDF-y i strony, które już masz |
| [Przenoszenie stron z Kindla](Exporting-pl) | Udostępnianie przez Wi-Fi, Efekty, PDF-y przez USB i SSH |
| [KOReader](KOReader-pl) | Otwieranie Grimoire z KOReadera, robienie notatek w książkach i opcjonalna wtyczka |
| [Skróty](Shortcuts-pl) | Klawiatury i kontrolery oraz `shortcuts.ini` |
| [Rozwiązywanie problemów](Troubleshooting-pl) | Znane problemy, typowe poprawki i przenoszenie między wersjami |
| [Prywatność](Privacy-pl) | Co zostaje na Kindle i co ujawnia Udostępnianie |
| [Współtworzenie](Contributing-pl) | Błędy, propozycje funkcji, pull requesty, korzystanie z LLM i wiki |
| [Budowanie ze źródeł](Building-from-Source-pl) | Zestawy narzędzi, cele make i testy |
| [Tłumaczenie](Translating-pl) | Ulepszanie hiszpańskiego, japońskiego i polskiego oraz dodawanie nowego języka |

## Wspieraj Grimoire

Grimoire jest darmowy, otwartoźródłowy i tworzony przez jedną osobę z dużą starannością. Jeśli
zasłużył na miejsce w twoim notatniku, możesz wesprzeć jego rozwój na Ko-fi:

<a href="https://ko-fi.com/B0B7EF4TJ"><img src="https://ko-fi.com/img/githubbutton_sm.svg" alt="Wesprzyj Grimoire na Ko-fi" height="36"></a>

Opinie i propozycje funkcji są mile widziane również w zgłoszeniach.

## Licencja

Grimoire to wolne oprogramowanie: GPL-3.0-or-later (zobacz `LICENSE`), ta sama licencja co
[FBInk](https://github.com/NiLuJe/FBInk) autorstwa NiLuJe, którego Grimoire używa do sterowania
ekranem e-ink. Wiele z tego, co Grimoire wie o Kindle, pochodzi z [KOReader](https://koreader.rocks)
i [kindlemodding.org](https://kindlemodding.org).

Kindle i Kindle Scribe są znakami towarowymi Amazon.com, Inc. lub jego spółek zależnych. Grimoire
jest niezależnym projektem i nie jest powiązany z Amazonem ani przez Amazon firmowany czy
wspierany.
