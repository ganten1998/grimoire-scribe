[English](Install) | [日本語](Install-ja) | **Polski**

# Instalacja

Grimoire działa na **Kindle Scribe z jailbreakiem**, który uruchamia skryplety (pliki `.sh`
uruchamiane dotknięciem w Bibliotece). Wszystkie obecne jailbreaki dla Scribe (Vera, SpiderCat,
WinterBreak, Sanctuary) to obsługują; jak wykonać jailbreak, przeczytasz na
[kindlemodding.org](https://kindlemodding.org). W przypadku starszego jailbreaka bez skrypletów
Grimoire możesz otworzyć przez KOReader (zobacz [KOReader](KOReader-pl)).

## Z archiwum ZIP wydania

1. Pobierz `Grimoire-<version>.zip` ze strony
   [Wydania](https://github.com/ganten1998/grimoire-scribe/releases) i rozpakuj. Otrzymasz folder
   `documents` z dwoma plikami oraz opcjonalne foldery wtyczki KOReadera i poprawek do ręcznej
   instalacji.
2. Podłącz Kindle do komputera. Scribe używa MTP: na Macu użyj
   [OpenMTP](https://openmtp.ganeshrvel.com) (lub `brew install --cask openmtp`); w Windows i
   Linuksie pojawi się jako urządzenie.
3. Skopiuj **oba pliki** do folderu `documents` na Kindle:
   - `Grimoire.sh`, aplikację (zawiera wszystko w środku)
   - `Grimoire - reset pen calibration.sh`, ratunkowy przełącznik, który, miejmy nadzieję, nigdy
     nie będzie potrzebny
4. Wysuń Kindle. **Grimoire** pojawi się w Bibliotece z gotycką okładką (indeksowanie może
   potrwać minutę). Dotknij go.

Pierwsze uruchomienie potrwa kilka sekund, podczas których aplikacja rozpakowuje się do folderu
`grimoire`. Twoje notatniki i ustawienia mieszkają tam, w `/mnt/us/grimoire` na Kindle.

## Aktualizacja

Skopiuj nowy `Grimoire.sh` na stary; notatniki i ustawienia zostaną. Powrót do starszej wersji
działa tak samo (zobacz [Rozwiązywanie problemów](Troubleshooting-pl#przenoszenie-między-wersjami)).

Możesz też aktualizować z aplikacji: **Biblioteka > Ustawienia > Aktualizacje** sprawdza kanał
Grimoire przez KPM i instaluje nowszą wersję. Grimoire się zamyka, KPM go instaluje (postęp na
ekranie), a Grimoire uruchamia się ponownie. Wersje beta dostają bety i wydania; dotknięcie tego
przełącznika przełącza na tylko wydania. Potrzebne są KPM i Wi-Fi.

## Przejście z Quire

Grimoire nazywał się Quire. Skopiuj nowe pliki i dotknij Grimoire raz. Przenosi notatniki i
ustawienia z `quire/` do `grimoire/` i usuwa stary launcher Quire.

## Instalacja przez KPM

Wydanie zawiera też pakiet KPM dla Scribe'ów `kindlehf`. Skopiuj `.kpkg` na Kindle, zamknij
Grimoire i w powłoce na Kindle uruchom:

```sh
/var/local/kmc/bin/kpm -y install file:///mnt/us/Downloads/grimoire_2.0.1001_kindlehf.kpkg
/var/local/kmc/bin/kpm launch grimoire
```

Zastąp nazwę pliku tym zasobem, który pobrałeś. Launchery w Bibliotece wywołują
`kpm launch grimoire`; cała aplikacja pozostaje w katalogu pakietów KPM i rozpakowuje się do
`grimoire/app` przy pierwszym uruchomieniu. Dołączony jest element resetowania kalibracji w
Bibliotece. Zamknij KOReader przed normalnym uruchomieniem z Biblioteki.

Instalacja przez KPM może przenieść dwa oficjalne launchery ręcznej instalacji Grimoire. Przy
odinstalowaniu usuwane są tylko niezmienione launchery KPM w Bibliotece; **wszystkie dane
`grimoire/` pozostają**, w tym notatniki, ustawienia, PDF-y i mapowania.

## Odinstalowanie

Usuń `Grimoire.sh` i `Grimoire - reset pen calibration.sh` z `documents`, a następnie folder
`grimoire` (najpierw skopiuj `grimoire/notebooks` lub `grimoire/PDFs` w inne miejsce, jeśli chcesz
zachować strony). Jeśli używasz KOReadera, usuń też `koreader/plugins/grimoire.koplugin`.

Z KPM: `kpm -y uninstall grimoire`. To usuwa tylko niezmienione launchery KPM w Bibliotece i
pozostawia wszystkie dane `grimoire/`; sam usuń folder z danymi tylko wtedy, gdy naprawdę chcesz
go skasować.
