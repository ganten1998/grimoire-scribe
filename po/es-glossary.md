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
| live (pressure and tilt) | **en tiempo real** | *En vivo* is said of broadcasts. |
| white ink, draws white | **tinta blanca**, **dibuja en blanco** (*Mantener presionado el botón del lápiz dibuja en blanco*; the test pad's *tinta blanca: pruébala sobre el negro*) | It takes away the ink under it and keeps the paper's lines. |
| Black / White (the tool picker's ink row) | **Negro / Blanco** | *Tinta negra / Tinta blanca* (they fit too). The colours as nouns, as a colour choice names them, not agreeing with *tinta*. |
| Eraser: Whole strokes / Precise (Pen feel) | **Borrador: Trazos enteros / Preciso** | *Por trazos / Exacto*. *Trazos enteros* as the eraser's note says it (*El borrador quita trazos enteros*); the precise one *quita solo lo que toca*. |

## Notebooks, papers, music

| English | Draft | Alternatives, notes |
| --- | --- | --- |
| notebook | **cuaderno** | *libreta*. A new notebook is named *Cuaderno 20 oct*. |
| Library | **Biblioteca** | |
| folder | **carpeta** | |
| Untitled | **Sin título** | The name of a notebook that has none (it is saved with it). |
| Template (a picture used as paper) | **Plantilla** | Copied into grimoire/templates; the paper's name under its picture. |
| More papers... (the last paper's tile, which opens the sheet of templates, *Plantillas*) | **Más papeles...** | *Otros papeles...* |
| Imported page, to import | **Página importada**, **importar** | A page of a document brought in as pictures (*Se importó un cuaderno de 12 páginas*). |
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
| Back to KOReader | **Volver a KOReader** | In Exit's place when Grimoire was opened from KOReader; this one button takes the width it needs. |
| Annotate this page in Grimoire (KOReader's Tools > Grimoire, in a book) | **Anotar esta página en Grimoire** | The plugin's own words (its table in kindle/koreader/grimoire.koplugin/main.lua): *anotar*, to write notes on it. *Comentar*. |
| the page KOReader sent (to annotate) | **la página que envió KOReader** (*No se pudo abrir la página que envió KOReader*) | *la página de KOReader* read as one of KOReader's own screens. |
| picture (of a page) | **imagen** (*no se puede leer su imagen*) | As the import folder's README says *imágenes*; *foto* is a photograph. |
| Done | **Listo** | *Hecho* (Spain), *Aceptar*. |
| Cancel, Retry | **Cancelar, Reintentar** | |
| Back (the Templates sheet's button back to the papers) | **Volver** | *Atrás*. |
| Delete | **Borrar** | *Eliminar*. A notebook or pages go (*Se borraron 3 páginas*). |
| Clear (a page, the test pad) | **Limpiar** (*Se limpió la página*) | The strokes go, the page stays: not *borrar*, which deletes. *Vaciar* (*Página vaciada*). |
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
| Live view, Stop | **Vista en directo**, **Detener** (*La vista en directo terminó*) | The screen watched in a computer's browser. The computer **mira** (*Un ordenador está mirando esta pantalla*). |
| Controller, its buttons, pair, forget | **mando**, **botones del mando**, **emparejar**, **olvidar** (*Mando conectado*, *Emparejar un mando*) | A game controller: *mando* (Spain), *control* in much of Latin America. A press: *pulsa* (*Pulsa un botón para Deshacer*). The Bluetooth helper: *asistente Bluetooth*. |
| A controller's actions (the Buttons sheet's rows): Erase while held, Eraser mode, Clear page, Fit page, Tool picker, Fold the panel, Close menu, Add a link, Name the page | **Borrar manteniendo presionado**, **Modo del borrador**, **Limpiar la página**, **Ajustar la página**, **Selector de herramientas**, **Plegar el panel**, **Cerrar el menú**, **Añadir un enlace**, **Nombrar la página** | Each name also fills *Pulsa un botón para %s* and *Ese botón ya no es para %s*, so they keep their articles (*Cerrar menú* reads as a label). *Add a link* there has the context `buttons`, so it keeps its article where the Index's button stays *Añadir enlace*; *Nombrar la página* is the open page's, as the Index's *Nombrar esta página*. The sheet's groups: **Edición, Herramientas, Páginas, Índice, Menús, Pantalla, Sistema** (the Index's group is *Índice...* in the list and *Índice* as the sheet's title). Previous / next: *Página anterior / siguiente*; in the tool picker: **Anterior / siguiente en el selector** (*Pulsa un botón para Siguiente en el selector*), the Tools group saying of what. |
| Link (drawn on a page), Add a link | **enlace**, **Añadir enlace** (*Enlazado con la página 3.*) | Where a link goes is where it **lleva** (*Cambiar a dónde lleva...*), or where it **va** in Link to...'s title (*¿Adónde va el enlace?*); the lasso around what links: *Rodea lo que enlaza.* Not *Vínculo*. (*Quitar enlace*, the Pages view's Remove link, is retired from 2.0.0-beta.6.) |
| A link's area (Index phase 2: Redraw the area, the strip, New area saved) | **zona** (*Volver a dibujar la zona*, *Dibuja la nueva zona del enlace.*, *Zona nueva guardada.*) | As *zona de enlace* below. The long-press card is never named in a string (no *tarjeta*); its rows are verbs: *Renombrar en el índice*, *Cambiar a dónde lleva...*, *Volver a dibujar la zona*, *Borrar enlace* (deleting a link is **borrar**, as Delete elsewhere). |
| Pin (pens to the panel), unpin | **Fijar** (*Fijar en el panel (hasta 4)*), **quitar** (*Quita una primero*) | As *Fijar esta página*; *Anclar*, *desanclar*. The pens pinned are *herramientas* (*En el panel caben hasta 4 herramientas*), as the tool picker's title calls them. |
| Style: Gothic / Classic | **Estilo: Gótico / Clásico** (*Estilo: Gótico (toca para cambiar a Clásico)*) | *toca para Clásico* had no verb; the other rows a tap changes say *toca para cambiar*. |
| Clock and battery (the indicator on a notebook page: the Buttons sheet's rows, and the Display sheet's size and corner cells) | **Reloj y batería**; **Tamaño del reloj: pequeño / mediano / grande**; **Esquina del reloj: arriba a la izquierda / arriba a la derecha / abajo a la derecha / abajo a la izquierda** (the toasts say it in full: *Reloj y batería: esquina superior izquierda de la página*) | Each of the Display sheet's two cells has one sentence per choice, so the adjective agrees with *tamaño*; at 34 px each has two lines of about 365 px. *Hora* would say less (the box has the date too). |
| Gesture (two fingers up, down, left, right, a tap), swipe | **gesto**, **deslizar** (*Aparta el lápiz y desliza*); the fingers are **dos dedos** (*Aquí dos dedos mueven la página*) | The pen toast speaks to the reader as tú, in the imperative, like the other toasts. A page turned past the end: *Es la última página*, *Es la primera página*. |
| Link area (the dotted boxes where a finger tap follows a link), Show link areas, Areas | **zona de enlace**, **Mostrar zonas de enlace**; the Links bar's tool button says **Áreas** (with *Ir a*, *Nuevo enlace*, *Índice*), 44 px caps words | The boxes are *zonas* in sentences and the button keeps the shorter *Áreas*. The Buttons sheet's Index actions: *Enlaces de esta página*, *Volver a la página anterior*, *Mostrar zonas de enlace*; *Página entera de lado* for sideways. |
| The Links bar (its header, its hint line, the peek and the way back, its chips) | **Enlaces de esta página (3)**, **Toca un enlace para mirarlo. Toca la página para quedarte.**, **Mostrando Árbol. Toca la página para quedarte.**, **‹ Página 3**, **Volver a Árbol**, **pág. 3**, **Página no disponible** | The bar looks at a page without going there: **mirar** and **mostrar** (*Mostrando*), then **quedarte** for staying on it. *pág.* is the chip's right column only (it has the context `bar`: the Pages view's *p. 3* stays as it is). A chip that has two links to one page says **×2**, the same in both languages. |
| Go to page or name (the Find sheet: the bar's *Ir a*, the Pages view's *Ir a la página...*) | **Número o nombre de página**, **Ir a la página 12**, **Ningún nombre coincide.**, **No hay página 9 (5 páginas).** | The number row is an action (*Ir a la página*, the Pages view's existing words); a name is shown as it is written, with accents and capitals ignored when searching. *No hay página 9 (1 página).* is the plural's singular form. |
| Sideways, show the whole page (the Display sheet's row) and its toasts | **De lado, ver la página entera**; *De lado se ve la página entera.*, *De lado ocupa todo el ancho.*, *Se aplica con el Kindle de lado.* | One line beside its box in 1300 px. The Display sheet is **Pantalla** in Spanish, which the update tip names (*se cambia en Pantalla*). |
| The one-time tips after an update (first open, sideways) and the eye's | **Novedad: dos dedos hacia arriba muestran los enlaces de esta página; hacia abajo, todas las páginas.**, **De lado ahora se ve la página entera y los gestos de dos dedos funcionan. Pellizca para ampliar; se cambia en Pantalla.**, **Los recuadros punteados muestran dónde un toque con el dedo sigue un enlace. Nunca se imprimen.** | Toasts of two lines at most (1700 px at 44 px in bold). The dotted boxes are *recuadros* in the tip and *zonas de enlace* everywhere else. |
| Language, Automatic | **Idioma, Automático** | The languages' own names stay in themselves (*English*, *Español*). |
| Calibrate | **Calibrar** (*Calibrar el lápiz y la pantalla táctil*) | |
| Calibrate the pen / Calibrate touch / Check it | **Calibrar el lápiz / Calibrar la pantalla táctil / Comprobar la calibración** | Calibration's three step titles, all infinitives with their object (was *Comprobación*; a bare *Comprobar* read like a button). |
| touch (the touchscreen) | **la pantalla táctil** | *el táctil* (Spain, informal). |
| target (calibration) | **diana** | *blanco* (could read as *white*), *objetivo*. |
| tap, hold, pinch | **tocar, mantener presionado, pellizcar** (*Vuelve a mantener presionado el botón de encendido 4 s*, *Mantener presionado el botón del lápiz borra*) | *Mantener* alone does not say the button stays down; Amazon's Spanish help writes *mantén presionado el botón de encendido*. *Mantener pulsado* in Spain. A tap is a **toque** (*Esos toques no tienen sentido*; *El toque del centro se desvió 2.3 mm*, the tap on the center target, not the center itself). |
| Mirror PDFs to grimoire/PDFs | **Copiar los PDF a grimoire/PDFs** | Folder names on the Kindle stay English (grimoire/PDFs, grimoire/trash, grimoire.log). |
| trash (the folder) | grimoire/trash | |
| storage | **almacenamiento** | |
| passcode | **contraseña** (*La contraseña del Kindle quedó desactivada*) | As the Kindle names it: Configuración > Opciones de dispositivo > *Contraseña del dispositivo*; Amazon's Spanish help says *contraseña de la pantalla de bloqueo*. Was *código de acceso*, which the Kindle does not use. |
| paused (the passcode, while Grimoire is open) | **suspendida** (*La contraseña del Kindle queda suspendida mientras Grimoire está abierto*) | Suspended for a while, and back when Grimoire closes. *En pausa* is for playback, downloads or a subscription, not a password; *desactivada* is the left-off notice's word, which asks the reader to turn it back on. |
| Kindle Settings | **la Configuración del Kindle** | The Kindle's own menus are *Configuración* and *Todas las configuraciones* (Amazon's Spanish help); capitalized as the menu's name, as the tip writes *ve a Ajustes*. Grimoire's own Settings stay *Ajustes*, which also keeps the two apart. Was *los ajustes del Kindle*. |
| defaults (Reset to defaults) | **valores predeterminados** (*Se restablecieron los valores predeterminados*) | The usual interface word; was *valores iniciales*. |
| a newer Grimoire | **una versión más reciente de Grimoire** (*Esta página se creó con una versión más reciente de Grimoire*) | *un Grimoire más nuevo* was a calque. The paper's short label stays *Papel más nuevo*. |

## The share pages and the studio

| English | Draft | Alternatives, notes |
| --- | --- | --- |
| Share over Wi-Fi | **Compartir por Wi-Fi** | *wifi* is the RAE's spelling; the Kindle writes Wi-Fi. |
| phone, computer | **teléfono, computadora** | *móvil*, *ordenador* (Spain); *celular* (Latin America). |
| browser | **navegador** | |
| Show pen (Live view's switch in the browser) | **Mostrar el lápiz** | The Scribe's pen, so *lápiz*, as everywhere (above). |
| Monitor mode (Live view's button in the browser: the page fitted, the controls tucked away, for drawing while looking at the computer) | **Modo monitor** (*Modo monitor (M)*) | *Monitor* is the computer's screen, which *pantalla* would confuse with the Kindle's. |
| Latency (the readout) | **Latencia** | Its lines say how late each thing shows: **retraso** (below), the plain word. |
| page image (Live view's exact picture of the page, the real ink that replaces the quick line; sent as tiles, which never show as such) | **la imagen** (*Retraso de la imagen en pantalla*) | *Bloques* (the tiles) meant nothing to a reader. |
| Pen age at paint, Page image age at paint | **Retraso del lápiz en pantalla**, **Retraso de la imagen en pantalla** | How late each shows on the computer, from when the Kindle read the pen. Was *Tiempo del lápiz al dibujar*, which read as the time spent drawing (*dibujar* is what the pen does). |
| p50 / p90 / max (the readout's three numbers) | **p50 / p90 / máx** | The median and the 90th percentile keep their usual short names. |
| Clock uncertainty | **Incertidumbre del reloj** | *Margen de error del reloj*. |
| video; Record (WebM) | **video**; **Grabar (WebM)** | *vídeo* (Spain). MP4 and WebM are format names, never translated. |
| download, served | **descargar, Descargas realizadas** | The counter is the downloads made; *Descargas servidas* read as a calque (food is served). *Descargas atendidas*. |
| Effects, studio | **Efectos, estudio** | |
| Presets | **Preajustes** | *Estilos*. |
| Original, Transparent, Signature, Warm, Sepia, Parchment, Blueprint, Chalkboard, Night, Sunset ink, Aurora | **Original, Transparente, Firma, Cálido, Sepia, Pergamino, Cianotipo, Pizarra, Noche, Tinta de atardecer, Aurora** | *Plano* for Blueprint. |
| Background, Gradient, Tint, Duotone | **Fondo, Degradado, Tinte, Duotono** | |
| Tone: None, Gray, Duotone, Invert | **Tono: Ninguno, Gris, Duotono, Invertido** | The tones are values of *Tono*, so adjectives that agree with it; was *Invertir*. The Display sheet's *Invertir* is an action and stays. |
| Shadows, Highlights | **Sombras, Luces** | |
| Trim to ink, Whole page | **Ajustar a la tinta, Página entera** | *Recortar a la tinta*. When it finds no ink, the info line says **sin tinta a la que ajustar**, echoing the choice (was *sin tinta que recortar*, which made the ink the thing cut). |
| Ink (your writing) | **Tinta (tu escritura)** | |
| cool 30 / warm 30 | **frío 30 / cálido 30** | |

## Numbers, dates, signs

- Decimals keep the point (*0.30 mm*), as the code writes them; Mexico and much of Central
  America write it so, Spain and most of South America write *0,30*. Please say which you expect.
- Percent: *150 %* with a space (as the RAE writes it), on the zoom chip and on Pen feel's Tilt
  widens value (*+35 %*).
- Angles: *45°* (the English writes *45 deg*).
- Dates: *20 oct, 14:32*; *sept* for September (the RAE's abbreviation; *sep* is also used).
- Quotes: «Cuaderno 20 oct» in the question before a notebook is deleted.
- No º or ª anywhere: the Gothic face draws them as a plain o and a.
- Not translated: Grimoire, Kindle, Kindle Scribe, Wi-Fi, PDF, PNG, ZIP, ImageMagick, macOS,
  Linux, the command `tools/grimoire-pull --http`, and the folder and file names above.


## Phase 3 gestures and action registry

| English | Spanish | Use and budget |
| --- | --- | --- |
| Gestures; gesture | **Gestos; gesto** | Settings bottom word is **Gestos**; gestures do not depend on **Activar atajos**. The row is *Gestos: activados/desactivados (toca para cambiar)*. |
| finger shape; finger shape (newer version) | **figura con el dedo; figura con el dedo (versión más nueva)** | Always *figura con el dedo* in a binding row, never bare *figura*. Smart shapes keep *forma*. A record-card sentence may shorten the repeated noun to *figura*. |
| tap; 2-finger tap | **toque; toque con 2 dedos** | Direction rows: *1 dedo hacia arriba*, *2 dedos hacia abajo*, *3 dedos a la izquierda*, *2 dedos a la derecha*. |
| no gesture | **sin gesto** | A binding's empty state, not *ninguno*. |
| Screen... / Screen; System... / System | **Pantalla... / Pantalla; Sistema... / Sistema** | Seven shared groups: Edición, Herramientas, Páginas, Índice, Menús, Pantalla, Sistema. |
| Do the gesture for %s | **Haz el gesto para %s** | Record card title; the substituted action is the registry's existing translated label/context. |
| Use; Try again; Reset gestures | **Usar; Otra vez; Restablecer los gestos** | Use saves; Try again starts a fresh attempt; Reset restores built-ins and deletes recorded shapes after confirmation. |
| Now for %s. Use moves it here. | **Ahora es para %s. Usar lo trae aquí.** | Names the action losing a pattern. Shape version: *Es la figura de %s. Usar la trae aquí.* |
| Lock rotation; Pinch to zoom; Switch theme | **Bloquear la rotación; Pellizcar para ampliar; Cambiar de tema** | Registry labels, independent of the longer Settings sentences. |
| Less / more light; less / more warmth | **Menos / más luz; Menos / más calidez** | Pair labels; individual steps keep *Menos luz*, *Más luz*, *Menos calidez*, *Más calidez*. |
| Zoom out / zoom in; Scroll up / down | **Alejar / acercar; Desplazar arriba / abajo** | Each member also has its individual registry label. |
| PDF copies; Sleep screen; Sleep now | **Copias en PDF; Pantalla de reposo; Reposo ahora** | The sleep-screen cycle retains KOReader wallpaper as its fourth choice. |
| Update channel; Automatic; Preview; Stable; Never | **Canal de actualizaciones; Automático; Beta; Estable; Nunca** | These short values fill action toasts. Studio's *Preview* keeps its separate context and translation *Vista previa*. |
| %s: %s; %s: %s / %s | **%s: %s; %s: %s / %s** | Both use context `gesture row`; preserve every placeholder and its order. |

New refusal/progress sentences follow PHASE3-BRIEF section 15. Keep tú imperatives (*Desliza*,
*Apoya*, *Dibújala*, *Muévete*). The 248 px / 34 px caps footer budget is checked for Controller
(**Mando**), Gestures (**Gestos**) and Updates (**Actualizaciones**) in English/Spanish and
Gothic/Classic. Existing translations and contexts remain intact; the compact footer did not
require abbreviating *Actualizaciones*. Record-card refusal lines are tested in two lines at
34 px within the actual 1292 px line rectangle, and the unwrapped footer within 1312 px.
