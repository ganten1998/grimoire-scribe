[English](Privacy) | [日本語](Privacy-ja) | **Polski**

# Prywatność

- **Kod dostępu urządzenia pozostaje włączony przy uruchomieniu z Biblioteki.** Grimoire nigdy go
  nie odczytuje ani nie zapisuje: wprowadza go klawiatura Kindle, a Grimoire nie rejestruje żadnego
  wejścia, gdy klawiatura ma ekran. To przekazanie wymaga jeszcze potwierdzenia na każdym
  firmware, a sesja KOReadera mogła wyłączyć kod dostępu; zobacz
  [Korzystanie z Grimoire](Using-Grimoire-pl#kod-dostępu).
- **Zgumowany tusz.** Całkowicie zgumowane pociągnięcia są usuwane przez kompaktowanie w tle po
  zamknięciu notatnika. Częściowo zgumowane pociągnięcie pozostaje w pliku strony w całości; stare
  eksportery mogą pokazać jego zgumowane fragmenty. Usunięte strony i notatniki pozostają w
  `grimoire/trash`, dopóki ich nie usuniesz.
- **Udostępnianie nie ma hasła.** Każdy w tej samej sieci może je otworzyć, dopóki ekran
  Udostępniania jest otwarty, i nikt po jego zamknięciu. Nie otwieraj Udostępniania w sieci,
  której nie ufasz.
- **`grimoire.log`**, o który prosi zgłoszenie błędu, nie zawiera numeru seryjnego, konta, nazwy
  sieci Wi-Fi ani adresu sieciowego, a Udostępnianie odnotowuje pobrania bez nazw notatników i
  folderów. Nigdy nie wymienia też twojej czcionki, ani po pliku, ani po rodzinie: tylko rozmiary,
  liczby i czasy.
- **Pliki z zewnątrz są czytane w piaskownicy.** PDF, zdjęcie lub czcionka skopiowane na Kindle są
  najpierw czytane przez oddzielną, zamkniętą kopię Grimoire, która nie ma sieci, nie widzi
  żadnych plików, działa jako własny użytkownik i może wywołać tylko krótką listę wywołań
  systemowych; to, co zgłosi, jest sprawdzane, zanim cokolwiek zostanie zapisane. Czcionka jest
  rysowana przez sam Grimoire dopiero po przejściu tego sprawdzenia dla dokładnie swoich bajtów.
- **Podgląd na żywo i Udostępnianie używają zwykłego HTTP**, tylko dopóki ich ekran jest otwarty
  i tylko w twojej sieci lokalnej. Używaj ich w sieci, której ufasz.
- **Hasła Wi-Fi nigdy nie trafiają do Grimoire.** Dodawaj i zmieniaj sieci w ustawieniach Kindle
  lub w KOReaderze; pomocnik Grimoire zachowuje tylko nazwy sieci.
- **Twoje czcionki zostają na Kindle.** Grimoire nigdy nie kopiuje twojej czcionki do strony, PDF-u
  ani nigdzie indziej i nigdy jej nigdzie nie wysyła.
