# Spanish glossary (po/es.po)

Grimoire's Spanish is one neutral `es` for Spain and Latin America, and a first draft: every
choice below is for a native speaker to confirm or change. Each line gives the English, the word
the draft uses, and the alternatives we weighed. Where a widget is narrow, its room is given in
pixels; `make l10n-check` says when a change no longer fits (it lists `fit[es]: ...` lines), and
build/shots/l10n/es-gothic/ and es-classic/ show every screen in Spanish.

## Voice

- **Tú, not usted**, everywhere: *toca*, *mantén*, *dibuja*, *abre*, *marca*, *ve a Ajustes*,
  *despierta el Kindle*. Most current interfaces (and Amazon's own Spanish) speak to the reader
  this way. If the Kindle's Spanish screens use usted, the whole file should follow them.
- Instructions address the reader (*Toca Nuevo para empezar uno*); buttons are infinitives
  (*Guardar*, *Renombrar*, *Empezar de nuevo*); notices are impersonal (*No se pudo guardar el
  PDF*, *Se borraron 3 páginas*).
- Neutral words where the regions differ; the regional ones are listed as alternatives.

## The Scribe's pen and the drawing tools

| English | Draft | Alternatives, notes |
| --- | --- | --- |
| pen (the Scribe's stylus) | **lápiz** | Amazon calls it *Lápiz* in Spanish, so every mention of the device uses it: *Calibrar el lápiz*, *Sensibilidad del lápiz*, *el botón del lápiz*, *No se encontró el lápiz*. |
| Pencil (tool) | **Grafito** | *Lápiz* would collide with the pen above (*Sensibilidad: Lápiz* reads as the pen's own setting); *Lápiz de grafito* is too long for the tool picker (284 px at 34 px). |
| Mechanical pencil | **Portaminas** | *Lapicero* (Mexico, Central America), *lapicera* in places. |
| Ballpoint | **Bolígrafo** | *Pluma* (Mexico), *birome* (Argentina, Uruguay), *lápiz pasta* (Chile), *lapicero* (Peru). |
| Fountain pen | **Estilográfica** | *Pluma estilográfica* is 288 px, wider than the tool picker's 284; *pluma fuente* (Mexico, Chile). |
| Fineliner | **Marcador fino** | *Rotulador fino* (Spain), *plumón fino* (Chile, Peru), *microfibra* (Argentina). |
| Marker | **Marcador** | *Rotulador* (Spain), *plumón* (Mexico, Chile, Peru). In Spain *marcador* is also a bookmark. |
| Highlighter | **Resaltador** | *Subrayador* or *fluorescente* (Spain), *marcatextos* (Mexico). |
| Eraser | **Borrador** | *Goma* (Spain). |
| Charcoal | **Carboncillo** | |
| Calligraphy pen | **Pluma de caligrafía**, *Caligrafía* in the tool picker | |
| nib | **plumín** | *punta*. |
| Pen feel (per-tool pressure and tilt) | **Sensibilidad** (*Sensibilidad: Bolígrafo*) | *Respuesta*, *Tacto*. |
| pressure, tilt | **presión**, **inclinación** | |
| deadzone | **zona muerta** | |
| grain, smoothing | **grano**, **suavizado** | |
| test pad | **zona de prueba** | |

## Notebooks, papers, music

| English | Draft | Alternatives, notes |
| --- | --- | --- |
| notebook | **cuaderno** | *libreta*. A new notebook is named *Cuaderno 20 oct*. |
| Library | **Biblioteca** | |
| folder | **carpeta** | |
| Untitled | **Sin título** | The name of a notebook that has none (it is saved with it). |
| rule, ruled | **rayado** | *pauta* (Spain), *raya* (Mexico). |
| Narrow / Medium / Wide rule | **Rayado fino / medio / ancho** | *Rayado estrecho* is clearer but too wide sideways, where it ran into the next paper's name. *angosto* (Latin America). |
| Narrow, no margin | **Fino, sin margen** (*Medio, sin margen*, *Ancho, sin margen*) | |
| Blank | **En blanco** | *Liso* (Spain). |
| Dot grid | **Cuadrícula de puntos** | *Punteado*. |
| Graph paper | **Cuadriculado** | *Cuadrícula*. |
| Cornell notes | **Notas Cornell** | *Método Cornell*. The headings printed on the page (Cues, Notes, Summary) stay English on purpose, so a page looks the same everywhere. |
| Sheet music | **Pentagramas** | *Papel pautado* (Spain). |
| Grand staff | **Sistema de piano** | *Gran pentagrama*, *pentagrama doble* (too wide sideways). |
| Guitar tab | **Tablatura de guitarra** | Goes on two lines under its picture. |
| 4-string bass | **Bajo de 4 cuerdas** | |
| Custom music | **Partitura a medida** | *Música a medida*, *pauta musical personalizada*. |
| Treble / Bass staff | **Clave de sol / Clave de fa** | |
| Alto / Tenor staff | **Clave de contralto / Clave de tenor** | *Clave de do en tercera / en cuarta* (no ordinals: the Gothic face draws º and ª as a plain o and a). |
| Drum staff | **Batería** | *Percusión*. |
| Plain staff | **Pentagrama sin clave** | *Pentagrama vacío*. |
| Tab, 4 lines | **Tablatura, 4 líneas** | |
| Lyric line | **Línea de letra** | |
| system (of staves) | **sistema** | |
| Newer paper | **Papel más nuevo** | A paper from a newer Grimoire, drawn blank. |

## Screens, buttons, actions

| English | Draft | Alternatives, notes |
| --- | --- | --- |
| Settings | **Ajustes** | *Configuración* is 208 px at 34 px, more than the top bar's buttons can grow to hold (188 px in Gothic). |
| Share | **Compartir** | The top bar's buttons widen to hold it in Gothic (150 px; 138 px fit inside a button's carved line). |
| Exit / New / Folder | **Salir / Nuevo / Carpeta** | |
| Done | **Listo** | *Hecho* (Spain), *Aceptar*. |
| Cancel, Retry | **Cancelar, Reintentar** | |
| Delete | **Borrar** | *Eliminar*. |
| Rename | **Renombrar** | *Cambiar nombre*. |
| Keep it | **Conservarlo** | |
| Remove folder | **Quitar la carpeta** | *Eliminar la carpeta*. |
| Tick, Select all / none | **marcar**, **Marcar todas / Desmarcar todas** | |
| Display | **Pantalla** | |
| Brightness, Warmth, Invert | **Brillo, Calidez, Invertir** | *Tono cálido* for warmth, if the Kindle says so. |
| Sleep | **Reposo** (*Reposo tras 10 min*) | *Suspensión*. |
| Sleep screen: ... | **En reposo: ...** (*En reposo: la página abierta*) | *Pantalla de reposo: ...* did not fit with the screensaver's line. |
| screensaver | **salvapantallas** | *protector de pantalla* (Latin America). |
| Pin this page | **Fijar esta página** | |
| Style: Gothic / Classic | **Estilo: Gótico / Clásico** | |
| Language, Automatic | **Idioma, Automático** | The languages' own names stay in themselves (*English*, *Español*). |
| Calibrate | **Calibrar** (*Calibrar el lápiz y la pantalla táctil*) | |
| touch (the touchscreen) | **la pantalla táctil** | *el táctil* (Spain, informal). |
| target (calibration) | **diana** | *blanco* (could read as *white*), *objetivo*. |
| tap, hold, pinch | **tocar, mantener, pellizcar** | *pulsar* (Spain), *presionar* (Latin America) for the power button. |
| Mirror PDFs to grimoire/PDFs | **Copiar los PDF a grimoire/PDFs** | Folder names on the Kindle stay English (grimoire/PDFs, grimoire/trash, grimoire.log). |
| trash (the folder) | grimoire/trash | |
| storage | **almacenamiento** | |
| passcode | **código de acceso** | As the Kindle's own Settings name it: please check. |
| Kindle Settings | **los ajustes del Kindle** | Whatever the Kindle's own menu is called in Spanish (*Configuración*, *Todos los ajustes*). |

## The share pages and the studio

| English | Draft | Alternatives, notes |
| --- | --- | --- |
| Share over Wi-Fi | **Compartir por Wi-Fi** | *wifi* is the RAE's spelling; the Kindle writes Wi-Fi. |
| phone, computer | **teléfono, computadora** | *móvil*, *ordenador* (Spain); *celular* (Latin America). |
| browser | **navegador** | |
| download, served | **descargar, Descargas servidas** | |
| Effects, studio | **Efectos, estudio** | |
| Presets | **Preajustes** | *Estilos*. |
| Original, Transparent, Signature, Warm, Sepia, Parchment, Blueprint, Chalkboard, Night, Sunset ink, Aurora | **Original, Transparente, Firma, Cálido, Sepia, Pergamino, Cianotipo, Pizarra, Noche, Tinta de atardecer, Aurora** | *Plano* for Blueprint. |
| Background, Gradient, Tint, Duotone | **Fondo, Degradado, Tinte, Duotono** | |
| Shadows, Highlights | **Sombras, Luces** | |
| Trim to ink, Whole page | **Ajustar a la tinta, Página entera** | *Recortar a la tinta*. |
| Ink (your writing) | **Tinta (tu escritura)** | |
| cool 30 / warm 30 | **frío 30 / cálido 30** | |

## Numbers, dates, signs

- Decimals keep the point (*0.30 mm*), as the code writes them; Mexico and much of Central
  America write it so, Spain and most of South America write *0,30*. Please say which you expect.
- Percent: *150 %* with a space on the zoom chip (as the RAE writes it); Pen feel's *+35%* is a
  value the code writes and stays as it is.
- Angles: *45°* (the English writes *45 deg*).
- Dates: *20 oct, 14:32*; *sept* for September (the RAE's abbreviation; *sep* is also used).
- Quotes: «Cuaderno 20 oct» in the question before a notebook is deleted.
- No º or ª anywhere: the Gothic face draws them as a plain o and a.
- Not translated: Grimoire, Kindle, Kindle Scribe, Wi-Fi, PDF, PNG, ZIP, ImageMagick, macOS,
  Linux, the command `tools/grimoire-pull --http`, and the folder and file names above.
