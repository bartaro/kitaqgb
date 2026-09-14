# Bibliotecas de KITAQGB

[English](README.md) | [日本語](README.ja.md) | **Español**

El [manual de las bibliotecas KITAQGB en español](https://bartaro.github.io/kitaq-docs/es/gb-library.html) contiene las explicaciones de cada función y sus ejemplos de código.

`wire3d_dmg` es el renderizador monocromo de líneas 3D para Game Boy. Para 128×96, utilice `wire3d_dmg_96.c`; para 128×120, `wire3d_dmg.c`. En ambos casos se usa la API `Wire3DDMG_*`. `wire3d` y `dmg3d` se mantienen como entradas de compatibilidad para esas dos resoluciones, respectivamente. Compile una sola entrada por ROM. `wire3d_cgb`, exclusivo de color, sigue siendo un renderizador independiente.

[Guía del renderizador](wire3d_dmg_guide.md) / [Guía en japonés](wire3d_dmg_guide_ja.md)

Este directorio contiene tres clases de archivos:

- API públicas: bibliotecas reutilizables cuya cabecera se incluye en el juego y cuyo código se compila cuando hace falta.
- Unidades auxiliares: apoyo opcional, declaraciones de registros o archivos de relleno que no constituyen por sí solos una API pública.
- Documentación de referencia: material de consulta que no se enlaza con la ROM.

## Clasificación de archivos

### API públicas

| Archivos | Función | Uso habitual |
| --- | --- | --- |
| `physics2d.h` / `physics2d.c` | Física 2D con cajas alineadas a los ejes (AABB), integración de gravedad y resolución iterativa de contactos. | Incluir `physics2d.h` y compilar `physics2d.c`. |
| `physics2d_circle.h` / `physics2d_circle.c` | Física 2D de cuerpos circulares, adecuada para juegos con bolas. | Incluir la cabecera y compilar el código. |
| `physics3d.h` / `physics3d.c` | Física 3D AABB con aceleración, rebote ponderado por masa, indicadores de rotura y `kq3d_dot_q8_8()`. | Incluir la cabecera y compilar el código. |
| `wire3d.h` / `wire3d.c` | Líneas 3D en punto fijo sobre un búfer de preparación en WRAM, con líneas ocultas del modelo y máscaras de oclusión de la escena. | Incluir `wire3d.h` y compilar `wire3d.c`. |
| `dmg3d.h` / `dmg3d.c` | Renderizado DMG con superficie de 128×120 en D000, líneas en ensamblador integrado con orden fijo y transferencia D000→8900 controlada por STAT. | Incluir la cabecera y el código para el flujo de dibujo preparado a 1 bpp. |
| `wire3d_cgb.h` / `wire3d_cgb.c` | Líneas 3D en color, exclusivas de CGB a 8 MHz: búfer WRAM de 2 bpp, líneas ocultas y oclusión, recorte en ensamblador y presentación sin desgarro mediante HBlank DMA. | Crear una ROM exclusiva de CGB e incluir la cabecera y el código. |
| `system.h` / `system.c` | Inicialización, contador de fotogramas, espera de VBlank, callbacks cooperativos de VBlank y envoltorios de DI/EI. | Bucles de juego organizados por fotogramas. |
| `input.h` / `input.c` | Estados de botones por fotograma: mantenido, recién pulsado, recién soltado y repetición. | Controles de menús y juegos de acción, puzles o estrategia. |
| `vram.h` / `vram.c` | Cola de comandos VRAM para escrituras de tiles BG, rellenos rectangulares, copias de bloques de mapa, memcpy y memset. | Encolar durante el juego y llamar a `vram_flush()` o `vram_flush_now()` en un periodo seguro. |
| `sprite.h` / `sprite.c` | Búfer espejo de OAM, asignación de sprites, metasprites, avance de animaciones, actualización por OAM DMA y detección de exceso por línea de barrido. | Representación basada en OBJ. |
| `fixed.h` / `fixed.c` | Punto fijo Q8.8, `Vec2`, `KQRect`, clamp/min/max/lerp y comprobaciones rectangulares básicas. | Movimiento, física, cámara y puntuaciones de IA, entre otros usos. |
| `scene.h` / `scene.c` | Tabla ligera de escenas con cambio, actualización y dibujo para estados como título, juego y pausa. | Organizar el flujo de estados del juego. |
| `entity.h` / `entity.c` | Reserva de objetos en un array fijo para un máximo de `ENTITY_MAX` entidades pequeñas. | Los callbacks reciben un ID; `entity_get(id)` permite acceder a sus datos. |
| `danmaku.h` / `danmaku.c` | Reserva de 96 proyectiles en punto fijo, abanicos de 32 direcciones, eventos de impacto y roce, y composición con tiles BG de CGB sin depender del límite de OAM. | Incluir cabecera y código; consultar `danmaku_guide.md` y el juego completo `ressen_gbc`. |
| `bank.h` / `bank.c` | Datos y punteros lejanos, llamadas lejanas y cambios sencillos de bancos MBC, basados en operaciones intrínsecas. | Envolver accesos entre bancos. |
| `asset.h` / `asset.c` | Tabla descriptiva de ID de recursos y carga de datos sin procesar o tiles. | Incluir cabecera y código; los futuros `assets.h/c/json` generados también pueden seguir este esquema. |
| `debug.h` / `debug.c` | Búferes ligeros de trazas, aserciones y marcas en la ROM para su inspección desde KOKURA u otro emulador. | Mantener el análisis de rendimiento costoso fuera de la ROM. |
| `chain.h` / `chain.c` | Búfer circular de posiciones anteriores para serpientes, cuerdas, trenes y sprites articulados. | Mover objetos segmentados siguiendo una trayectoria registrada. |
| `cgb_tile.h` | Declaraciones públicas de las operaciones intrínsecas de tiles y atributos CGB. | Incluir cuando el juego utilice `__settile...` y funciones similares. |
| `cgb_palette.h` / `cgb_palette.c` | Interfaz de alto nivel para paletas BG/OBJ de CGB. | Incluir la cabecera y compilar el código. |
| `scroll.h` / `scroll.c` | Desplazamiento y tablas de pantalla dividida mediante operaciones intrínsecas del compilador. | Añadir al utilizar `Scroll_*`. |
| `raster.h` / `raster.c` | Desplazamiento por bandas y deformación horizontal por línea de barrido. | Incluir `raster.h` y compilar tanto `raster.c` como `scroll.c`. |
| `camera.h` / `camera.c` | Cámara en punto fijo 8.8 sobre `scroll.*`, interfaz global sencilla y conversión entre coordenadas del mundo y de pantalla. | Incluir la cabecera y compilar el código. |
| `audio.h` / `audio.c` | Controlador de audio común para Game Boy: música, efectos, panoramización, ondas y fundidos, con 68 índices de nota, del 0 al 67 (`G6`). | Añadir a proyectos que necesiten audio. |
| `audio_vblank.h` / `audio_vblank.c` | Controlador BGM por IRQ de VBlank, también con 68 índices de nota, cola WRAM de 16 registros para canciones entre bancos y un punto de enganche por fotograma. | Situar canciones con puntero directo en el banco fijo 0 o alimentar la cola desde otros bancos; configurar el vector `0x0040` con `scripts/patch_gb_vblank_irq.ps1`. |
| `link.h` / `link.c` | Transferencia serie de bytes por cable y comunicación lógica cooperativa para cuatro jugadores con `Link4_*`. | Añadir a proyectos con comunicación. |
| `link_packet.c` | Capa opcional de paquetes sobre `link.c`, con buzones por interlocutor para `Link4_*`. | Compilar junto con `link.c` solo si se necesitan paquetes. |
| `link_dmg07.h` / `link_dmg07.c` | Controlador por sondeo y reloj externo para el adaptador físico Nintendo DMG-07 Four Player Adapter. | Compilar con `link_hwregs_gb.c`; es independiente de la API lógica `Link4_*`. |
| `rpg.h` | Declaraciones compartidas RPG/ADV/SLG y de operaciones intrínsecas de bajo nivel. | Incluir en el juego al utilizar esta familia de funciones. |
| `rng.c` | `rng8`, `rng16`, `rand_range`, `weighted_choice`, `rng_seed`, `rng_next8`, `rng_next16`, `rng_range`, `rng_chance`. | Compilar al utilizar las funciones aleatorias de `rpg.h`. |
| `flags.c` | Conjunto de 2048 bits de indicadores y almacenamiento del estado de misiones. | Compilar al utilizar indicadores o misiones de `rpg.h`. |
| `rle.c` | Decodificación RLE sencilla de pares `[cantidad][valor]` desde RAM o ROM lejana. | Compilar al utilizar `rle_decode*` de `rpg.h`. |
| `text.c` | Ventanas de cadenas de tiles, espera entre páginas, opciones, texto XY directo, números y alias de limpieza y ventana. | Compilar al utilizar las funciones de texto de `rpg.h`. |
| `menu.c` | Menús verticales, menú mínimo de objetos y API de estado de menú no bloqueante. | Compilar al utilizar las funciones de menú de `rpg.h`. |
| `script.c` | Pequeño ejecutor de bytecode para secuencias RPG/ADV. | Compilar al utilizar los scripts de `rpg.h`. |
| `map.c` | Carga de mapas empaquetados con colisiones, activadores, cámara y metatiles opcionales de 16×16. | Compilar al utilizar las funciones de mapa de `rpg.h`. |
| `save.c` | Guardado, carga, comprobación y borrado de SRAM al estilo MBC5, con cabecera, versión, longitud y suma de comprobación. | Compilar al utilizar las funciones de guardado de `rpg.h`. |
| `slg_unit.c` | Alcances de movimiento y ataque para juegos de estrategia. | Compilar al utilizar unidades tácticas de `rpg.h`. |
| `slg_path.c` | Búsqueda de rutas en anchura y propagación de costes de movimiento. | Compilar al utilizar las rutas tácticas de `rpg.h`. |
| `slg.h` / `slg_board.c` | Tableros, listas de jugadas y pilas de deshacer para sistemas tácticos o juegos de mesa. | Incluir `slg.h`, compilar `slg_board.c` y mantener aparte la evaluación específica del juego. |

### Unidades auxiliares

| Archivos | Función | Observaciones |
| --- | --- | --- |
| `audio_hwregs_gb.c` | Declaraciones mínimas de registros APU y RAM de ondas. | Utilizar solo si otro archivo no declara ya esos registros. |
| `link_hwregs_gb.c` | Declaraciones mínimas de `SB`, `SC`, `IF` e `IE` para comunicación. | No añadir si los registros serie ya están declarados en otro archivo. |
| `cgb_tile.c` | Unidad de compilación intencionadamente vacía para las funciones de tiles CGB. | La interfaz está en `cgb_tile.h`; compilar esta unidad no perjudica, pero no es necesario para la lógica de ejecución. |
| `math.c` | Tabla de senos en ROM `MATH_SIN`. | Aún no se documenta como API pública estable; se usa como unidad de datos auxiliar del proyecto. |

### Documentación de referencia

| Archivo | Contenido |
| --- | --- |
| `README.md` | Versión inglesa de esta descripción y de las instrucciones de compilación. |
| `wire3d_guide_ja.md` | Guía introductoria en japonés del renderizador de líneas 3D. |
| `dmg3d_guide_ja.md` | Guía introductoria en japonés del renderizador DMG con búfer de preparación. |
| `wire3d_cgb_guide.md` | Guía introductoria del renderizador de líneas en color exclusivo de CGB. |
| `physics_guide.html` | Guía de las bibliotecas físicas en inglés. |
| `physics_guide_ja.html` | Guía de las bibliotecas físicas en japonés. |

## Cómo compilar

Algunos comandos siguientes hacen referencia a antiguas demostraciones de desarrollo que no están en el repositorio público, como `wire3d_cube_demo.c`. Son plantillas de compilación para usar cuando se disponga del código correspondiente. Para los programas introductorios incluidos, utilice `../examples/build.ps1` y el manual HTML. El comando abreviado `kitaqgb` presupone que el ejecutable está en PATH.

Compile el código del juego junto con las bibliotecas necesarias:

```powershell
kitaqgb hwregs.c lib/audio.c main.c lib/physics2d.c lib/physics2d_circle.c lib/physics3d.c lib/cgb_palette.c lib/scroll.c lib/camera.c -I lib -o game.gb --profile=dev
```

Los proyectos de líneas 3D deben compilar también el código del renderizador:

```powershell
.\kitaqgb.exe lib/wire3d.c examples/wire3d_minimal.c -I lib -o examples/wire3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

El ejemplo incluido `examples/wire3d_minimal.c` inicializa todos los campos del modelo y hace girar un cubo dentro de un área de 128×96, sin gráficos ni tipografías externos.

Los proyectos DMG con un área de 128×120 pueden utilizar la entrada de compatibilidad de 120 líneas, `dmg3d.*`:

```powershell
.\kitaqgb.exe lib/dmg3d.c examples/dmg3d_minimal.c -I lib -o examples/dmg3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

`DMG3D_Init()` configura una superficie visible de líneas de 128×120, con preparación en WRAM desde D000 y carga de tiles desde 0x8900. `DMG3D_BeginFrame()` solo reinicia la oclusión; la carga consume y borra los píxeles. `DMG3D_EndFrame()` espera primero a VBlank y después consulta STAT durante la transferencia, que puede prolongarse más allá de VBlank. El ejemplo `examples/dmg3d_minimal.c` activa la transferencia diferencial y vuelve a dibujar una cruz en cada fotograma. La transferencia auxiliar es una operación independiente que comparte almacenamiento de origen con el búfer principal.

Los proyectos de color exclusivos de CGB utilizan `wire3d_cgb.*`:

```powershell
kitaqgb lib/wire3d_cgb.c examples/wire3d_cgb_color_demo.c -I lib -o examples/wire3d_cgb_color_demo.gbc --profile=dev --stack-bank=fixed --rst-disable --cgb=cgb_only --rom-title=CGBWIRE3D
```

`Wire3DCGB_Init()` activa la doble velocidad de CGB, configura una superficie BG de 128×96 a 2 bpp e instala una paleta BG predeterminada de cuatro entradas. Tanto las API de fotograma normales como las `Fast` presentan la imagen sin desgarro: HBlank DMA transfiere los 3072 bytes de `0xD300–0xDEFF` al banco de tiles VRAM que no está visible, y la presentación cambia durante VBlank. No hace falta modificar LCDC en cada fotograma. Las variantes `Fast` omiten la comprobación de la cola BG que realiza el cierre normal. Los colores se controlan con `Wire3DCGB_SetPaletteRGB15()`, `Wire3DCGB_SetLineColor()` y `Wire3DCGB_Draw*Color()`.

Para niveles de detalle por orientación generados con CAD, `Wire3DCGB_DrawMaskedModel2D()` acepta desplazamientos de vértices con signo ya proyectados y una máscara empaquetada de aristas visibles. El recorrido de aristas y la rasterización en ensamblador se realizan dentro del banco 4 del renderizador, por lo que dibujar un modelo no exige una llamada entre bancos por cada línea.

En juegos con pocos cambios de imagen y búfer espejo de OAM, puede llamar a `sprite_flush_oam()` y después a `Wire3DCGB_EndFrameSparseNow()` una vez iniciado VBlank. Se omite la espera inicial al siguiente VBlank, pero DMA y el cambio de presentación aún pueden esperar según la región modificada y la línea actual. No se garantiza que todo termine en el mismo VBlank.

Utilice los colores 1, 2 y 3 para las líneas CGB. El modo normal de 128×96 combina los bits de color: al superponer 1 y 2 se obtiene 3. El color 0 no borra líneas; limpie el fotograma o utilice una función específica de borrado. `Wire3DCGB_DrawLine2D` y el dibujo normal de modelos no registran las regiones de transferencia diferencial. Para dibujar y registrar el área, use `Wire3DCGB_DrawLineClipped2D`, o llame a `Wire3DCGB_InvalidateFrameHistory` para incluir toda la superficie en la siguiente transferencia diferencial.

El modo de 160×144 puede asignar como máximo 127 tiles por fotograma. Si la ruta rápida de líneas agota la asignación o recibe coordenadas fuera de rango, activa `Wire3DCGB_GetFullScreenOverflow()` y deja de escribir píxeles hasta el reinicio del siguiente fotograma. Mantenga los vértices dentro del área elegida. La ampliación derecha de las máscaras triangulares se detiene en X=127 para 128×96 y en X=159 para pantalla completa. Respete los requisitos de mapeo de bancos WRAM de la API, especialmente en pantalla completa y FastMap.

La [prueba de regresión de límites de máscaras triangulares CGB](../tests/library/wire3d_cgb_mask_bounds.c) ofrece un programa completo que comprueba ambas áreas.

La antigua demostración `examples/wire3d_cgb_hiddenline_demo.c` permite comprobar las líneas ocultas de forma interactiva. `START` alterna entre uno y tres objetos visibles; `B` selecciona un objeto; la cruceta mueve X/Y; `A`+arriba/abajo mueve Z; `A`+izquierda/derecha gira Z; y `SELECT`+cruceta gira X/Y en pasos de 22,5 grados. Las líneas ocultas y la oclusión entre objetos permanecen activas, y los objetos que colisionan se empujan. Esta demostración de desarrollo no se incluye en el repositorio público.

Ejemplo de compilación para funciones RPG/ADV/SLG:

```powershell
kitaqgb examples/example_rpg_text.c lib/text.c lib/menu.c -I lib -o text.gb --profile=dev
kitaqgb examples/example_adv_script.c lib/text.c lib/flags.c lib/script.c -I lib -o script.gb --profile=dev
kitaqgb examples/example_slg_cursor.c lib/map.c lib/slg_unit.c lib/slg_path.c -I lib -o slg.gb --profile=dev
```

Ejemplo de compilación de la prueba básica del entorno de ejecución estándar:

```powershell
kitaqgb lib/text.c lib/menu.c lib/map.c lib/scroll.c lib/camera.c lib/rng.c lib/save.c lib/system.c lib/input.c lib/vram.c lib/sprite.c lib/fixed.c lib/scene.c lib/entity.c lib/bank.c lib/asset.c lib/debug.c lib/chain.c lib/physics2d.c lib/slg_board.c examples/standard_library_smoke.c -I lib -o examples/standard_library_smoke.gb --profile=dev --rom-title=STDLIBSMK --no-disasm
```

En proyectos de comunicación serie, compile la unidad de declaraciones de registros antes que el núcleo de comunicación:

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c main.c -I lib -o game.gb --profile=dev
```

Los proyectos cooperativos de cuatro jugadores lógicos, donde el anfitrión elige al interlocutor, utilizan los mismos archivos. El anfitrión llama a `Link4_InitHost(slot_count)` y selecciona el interlocutor con `Link4_SelectPeer()` o `Link4_SendPacketTo()`. Los demás llaman a `Link4_InitPeer(local_slot, slot_count)` y se comunican con el anfitrión del puesto 0.

También puede compilar directamente los ejemplos de participantes con un puesto predefinido:

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot1.c -I lib -o peer1.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot2.c -I lib -o peer2.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot3.c -I lib -o peer3.gb --profile=dev
```

Para el adaptador DMG-07 físico, utilice el controlador de sondeo específico:

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link_dmg07.c main.c -I lib -o dmg07.gb --profile=dev
```

Llame continuamente a `LinkDmg07_Poll()`: el intervalo entre bytes del adaptador es mucho menor que un fotograma de vídeo. Llame una vez por VBlank a `LinkDmg07_TickFrame()` para actualizar los contadores saturados de silencio y de tiempo de espera del establecimiento de conexión. El controlador siempre usa reloj externo, `SC=$80`, responde a la detección con `88 88 RATE 01` y solo permite al jugador físico 1 solicitar una transferencia mediante `AA AA AA AA`.

Cuando todas las consolas han observado `CC CC CC CC`, cada paquete de difusión de cuatro bytes contiene un byte de cada puesto físico. El adaptador difunde los datos en el paquete siguiente al de su entrega, por lo que el controlador descarta el primer paquete de contenido indefinido y proporciona números de secuencia de envío y recepción. `LinkDmg07_RequestRestart()` espera al siguiente límite de paquete, transmite `FF FF FF FF` alineado y se detiene tras recibir la indicación completa de cuatro FF del adaptador. Un tiempo de silencio excesivo durante la transferencia también programa este reinicio, conservando la posición dentro del paquete de cuatro bytes. Cuando vuelve el reloj, puede terminarse ese paquete antes de iniciar la recuperación.

Incluya después las cabeceras necesarias en el juego:

```c
#include "physics2d.h"
#include "physics2d_circle.h"
#include "physics3d.h"
#include "wire3d.h"
#include "cgb_tile.h"
#include "cgb_palette.h"
#include "scroll.h"
#include "raster.h"
#include "camera.h"
#include "audio.h"
#include "audio_vblank.h"
#include "system.h"
#include "input.h"
#include "vram.h"
#include "sprite.h"
#include "fixed.h"
#include "scene.h"
#include "entity.h"
#include "bank.h"
#include "asset.h"
#include "debug.h"
#include "chain.h"
#include "slg.h"
```

## Notas de uso

Los últimos ocho índices de nota reutilizan actualmente las frecuencias de la octava anterior. Admitir 68 índices no significa producir 68 alturas distintas.

- `inv_mass_q8 == 0` indica un cuerpo estático.
- `Wire3D_Init()` utiliza una superficie BG de líneas de 128×96, un búfer WRAM desde `0xD000` y datos de tiles desde `0x8900`. En proyectos Wire3D, use `--stack-bank=fixed`.
- `Wire3D_BeginFrame()` vacía el búfer WRAM; `Wire3D_EndFrame()` espera a VBlank y copia bloques a VRAM según las condiciones de STAT.
- Wire3D utiliza 16 posiciones angulares. La ruta básica admite hasta `WIRE3D_MODEL_VERTEX_LIMIT` vértices por modelo.
- Para varios objetos de líneas con caras que se superponen, utilice `Wire3D_DrawScene()`. Dibuja primero los cercanos, acumula máscaras de caras visibles y omite de forma conservadora las líneas lejanas ocultas.
- `Wire3DCGB_Init()` solo admite CGB y activa la doble velocidad mediante KEY1/STOP. Compile con `--cgb=cgb_only` y no mezcle `wire3d_cgb.*` con `wire3d.*`, compatible con DMG, en la misma ROM.
- Las API de fotograma normales y `Fast` conservan la imagen anterior hasta terminar la transferencia al banco de tiles VRAM no visible. Prefiera `Fast` en escenas que no utilicen la cola de escrituras de tiles BG.
- El archivo que declare `NR10..NR52` y `WAVE0..WAVE15` debe compilarse antes de `lib/audio.c`.
- `lib/audio_hwregs_gb.c` ya proporciona esas declaraciones; no lo compile junto con otro archivo que declare los mismos registros de audio.
- `cgb_tile.h` expone directamente las operaciones intrínsecas. `lib/cgb_tile.c` es solo una unidad de relleno que puede omitirse en una compilación normal.
- La API pública de `cgb_palette.h` utiliza nombres `cgb_*`.
- Cuando un menú o un ajuste active o desactive música o efectos, llame a `Audio_SetMusicEnabled()` / `Audio_SetSfxEnabled()`.
- `Audio_PlaySFX()` registra el banco ROM visible en ese momento. Si conoce el banco de los datos del efecto, utilice `Audio_PlaySFXBanked(bank, sfx, priority)`.
- `AUDIO_CMD_NOTE` / `AUDIO_CMD_SET_INST` conservan la numeración histórica de canales del flujo musical: `0=CH1`, `1=CH2`, `2=CH4`, `3=CH3`.
- Para una onda CH3 propia, empaquete 32 muestras de 4 bits en 16 bytes y páselas a `Audio_LoadCustomWave()`.
- El fundido de `Audio_FadeToMasterVolume()` avanza mediante `Audio_Update()`: siga llamando a esta función en cada fotograma durante el fundido.
- `audio_vblank.c` define el símbolo de vector IRQ de VBlank `__kq_vblank_vector`. Cada evento BGM tiene cinco bytes: `delay, ch2_note, ch1_note, ch3_note, ch4_noise_param`. El silencio, el bucle y el final se indican con `AUDIO_VBLANK_REST`, `AUDIO_VBLANK_LOOP` y `AUDIO_VBLANK_END`.
- Las canciones VBlank con puntero directo deben estar en un banco fijo; el modo de cola puede recibir datos de otros bancos. Tras enlazar `lib/audio_vblank.c`, ejecute `scripts/patch_gb_vblank_irq.ps1 <rom> <map>` para dirigir el vector `0x0040` a la ISR y actualizar la suma de comprobación de la ROM.
- Sin un despachador IRQ compartido, no combine `audio_vblank.c` con otra biblioteca o rutina de entrada del juego que también ocupe el vector VBlank `0x0040`.
- `Scroll_SplitCommit()` habilita automáticamente los bits de IE `0x01 | 0x02` y utiliza los manejadores VBlank/STAT del compilador para aplicar la configuración de pantalla dividida.
- La pantalla dividida reserva actualmente los vectores `0x0040` y `0x0048`; por ahora, no la mezcle con rutinas de entrada VBlank/STAT propias e independientes.
- Las declaraciones de `SB`, `SC`, `IF` e `IE` deben compilarse antes de `lib/link.c` / `lib/link_packet.c` o de `lib/link_dmg07.c`.
- `lib/link_hwregs_gb.c` ya incluye esas declaraciones; no lo combine con otro archivo que declare los mismos registros serie.
- La biblioteca de comunicación no ocupa el vector serie `0x0058`. Si activa el modo por interrupciones, llame a `Link_OnSerialIRQ()` desde su rutina de entrada o despachador IRQ.
- La capa de paquetes solo mantiene un nivel de datos pendientes de recepción; el bucle principal debe atenderla con regularidad en cada fotograma.
- `Link4_*` modela una conexión cooperativa de cuatro jugadores en la que el anfitrión elige al interlocutor. Solo hay uno activo en el enlace a la vez, por lo que el anfitrión debe alternarlos explícitamente.
- `Link4_TryReadByteFrom()` / `Link4_HasPacketFrom()` ofrecen buzones por interlocutor para conservar el origen de los datos al sondear varios participantes.
- `Link_ReadPacket()` conserva la vista histórica del «paquete más reciente». En el flujo de cuatro jugadores, utilice `Link4_ReadPacketFrom()`.
- `Link4_*` no implementa el comportamiento eléctrico ni el protocolo del Nintendo DMG-07. Para el accesorio físico, use `link_dmg07.c` y no lo compile junto con `link.c` en la misma ROM.
- En DMG-07, `GetConnectedMask()` representa a los jugadores físicos 1–4 con los bits 0–3. Durante la transferencia conserva la última detección de conexión; los participantes solo se actualizan en la fase de detección.
- Un reinicio DMG-07 pendiente no puede avanzar sin reloj del adaptador. El tráfico de recuperación se descarta y no se entrega como datos de secuencia normales. Si el accesorio se vuelve a encender en una fase distinta, en vez de pausar únicamente su reloj, reinicialice expresamente el controlador y la sesión.
- Las bibliotecas físicas calculan posiciones y velocidades lineales; no incluyen dinámica angular.
- Limite el número de cuerpos para hardware de la clase Game Boy; por ejemplo, mantenga entre 8 y 24 cuerpos activos.
- Ajuste la gravedad, la velocidad máxima y las iteraciones del solucionador según las necesidades de cada mundo de juego.
- Para juegos de billar, prefiera `physics2d_circle.*` a la biblioteca AABB.
- La estructura actual no necesita bibliotecas adicionales `random`, `collision`, `ui`, `tilemap`, `dialog`, `board_game` ni `simple_physics`. Utilice, respectivamente, `rng`, `physics2d`, `text`/`menu`, `map`, `script`, `slg` y `physics2d`.
- `scene.c` y `entity.c` evitan argumentos del tamaño de un puntero en llamadas mediante punteros a función. La ruta actual de KITAQGB resulta más fiable sin argumentos o con ID de un byte.
