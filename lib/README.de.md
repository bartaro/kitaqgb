# KITAQGB-Bibliotheken

[English](README.md) | **Deutsch**

**[Deutsches Bibliothekshandbuch öffnen](https://bartaro.github.io/kitaq-docs/de/gb-library.html)**

`wire3d_dmg` ist eine Bibliothek für monochrome Drahtgittergrafik auf dem Game Boy. Wählen Sie `wire3d_dmg_96.c` für 128 × 96 oder `wire3d_dmg.c` für 128 × 120 und verwenden Sie `Wire3DDMG_*`. `wire3d` und `dmg3d` bieten alternative Einstiegspunkte für die Profile mit 96 beziehungsweise 120 Zeilen. Kompilieren Sie pro Programm nur einen Einstiegspunkt. `wire3d_cgb` ist der Renderer für die Farbdarstellung.

[Renderer-Anleitung auf Englisch](wire3d_dmg_guide.md) / [日本語](wire3d_dmg_guide_ja.md)

Dieser Ordner enthält drei Arten von Dateien:

- **Öffentliche APIs:** wiederverwendbare Bibliotheken, deren Header und Implementierungen Sie in Spielprojekte einbinden.
- **Hilfseinheiten:** optionale Unterstützung, Registerdeklarationen oder Platzhalter, die für sich keine gleichwertigen öffentlichen APIs bilden.
- **Referenzdokumente:** Anleitungen und Übersichten, die nicht in die ROM gelinkt werden.

## Einteilung

### Öffentliche APIs

| Dateien | Aufgabe | Übliche Verwendung |
| --- | --- | --- |
| `physics2d.h` / `physics2d.c` | 2D-AABB-Körper, Schwerkraftintegration und iterativer Kontaktlöser. | Header einbinden und Implementierung mitkompilieren. |
| `physics2d_circle.h` / `physics2d_circle.c` | Physik kreisförmiger Körper für Ballspiele. | Header einbinden und Implementierung mitkompilieren. |
| `physics3d.h` / `physics3d.c` | 3D-AABB-Physik mit Beschleunigung, massengewichteten Abprallreaktionen, Bruchmarkierungen und `kq3d_dot_q8_8()`. | Header einbinden und Implementierung mitkompilieren. |
| `wire3d.h` / `wire3d.c` | Drahtgitterrenderer mit Festkommaarithmetik, WRAM-Puffer und Verdeckung von Modellkanten und Szenen. | Header und Quelle verwenden, wenn das kompatible 96-Zeilen-Profil benötigt wird. |
| `dmg3d.h` / `dmg3d.c` | DMG-Drahtgitterrenderer mit 128 × 120-Puffer ab D000, Inline-Assembler und STAT-gesteuerter Übertragung nach 8900. | Für den kompatiblen 120-Zeilen-Pfad mit 1bpp-Puffer einbinden. |
| `wire3d_cgb.h` / `wire3d_cgb.c` | CGB-exklusiver Farbrenderer bei doppelter Taktrate, mit 2bpp-WRAM-Puffer, Verdeckung, API zur Linienbegrenzung und HBlank-DMA-Präsentation. | Header und Quelle mit einem CGB-exklusiven ROM-Build verwenden. |
| `system.h` / `system.c` | Kleine GB-Laufzeitbasis: Initialisierung, Bildzähler, VBlank-Warten, kooperativer VBlank-Callback und DI/EI. | Für eine am Bildwechsel ausgerichtete Spielschleife einbinden. |
| `input.h` / `input.c` | Eingabezustand pro Bild: gehalten, gedrückt, losgelassen und Wiederholung. | Für Menüs, Action-, Puzzle- und Strategiespiele einbinden. |
| `vram.h` / `vram.c` | VRAM-Warteschlange für BG-Kacheln, Rechtecke, Kartenblöcke, Kopieren und Füllen. | Änderungen vormerken und `vram_flush()` beziehungsweise `vram_flush_now()` in einem sicheren Zeitfenster aufrufen. |
| `sprite.h` / `sprite.c` | OAM-Schattenpuffer, Sprite-Reservierung, Metasprites, Animation, OAM-DMA und Prüfung der Scanline-Grenzen. | Für OBJ-basierte Darstellung einbinden. |
| `fixed.h` / `fixed.c` | Q8.8-Hilfen, `Vec2`, `KQRect`, Begrenzung, Minimum/Maximum, Interpolation und einfache Rechtecktests. | Für Bewegung, Physik, Kamera und KI-Bewertung einbinden. |
| `scene.h` / `scene.c` | Kleine Szenentabelle mit Wechsel-, Aktualisierungs- und Zeichenfunktionen, etwa für Titel, Spiel und Pause. | Zur Strukturierung des Spielzustands einbinden. |
| `entity.h` / `entity.c` | Fester Objektpool für bis zu `ENTITY_MAX` kleine Spielobjekte. | Callbacks erhalten eine Objekt-ID und können `entity_get(id)` verwenden. |
| `danmaku.h` / `danmaku.c` | Festkommapool mit 96 Geschossen, Fächern in 32 Richtungen, Treffer-/Streifereignissen und CGB-BG-Komposition unabhängig vom OAM-Limit. | Header und Quelle einbinden; siehe `danmaku_guide.md`. `ressen_gbc` ist ein historisches Entwicklungsbeispiel und gehört nicht zu diesem öffentlichen Paket. |
| `bank.h` / `bank.c` | Daten, Zeiger und Aufrufe über Bankgrenzen sowie einfache MBC-Umschaltung auf Basis der Intrinsics. | Für bankübergreifende Datenzugriffe einbinden. |
| `asset.h` / `asset.c` | Kleine Ressourcentabelle mit IDs und Hilfen zum Laden roher Daten oder Kacheln. | Header und Quelle einbinden; spätere Erzeuger für `assets.h/c/json` können dieses Format verwenden. |
| `debug.h` / `debug.c` | Kleiner ROM-seitiger RAM-Puffer für Ablaufmarkierungen und Assertions. | Mit KOKURA oder im Emulator auswerten; aufwendiges Profiling außerhalb der ROM durchführen. |
| `chain.h` / `chain.c` | Ringpuffer mit Koordinatenverlauf für Schlangen, Seile, Züge oder verbundene Sprites. | Für segmentierte Bewegungen einbinden. |
| `cgb_tile.h` | Deklarationen der Compiler-Intrinsics für CGB-Kacheln und Attribute. | Bei Verwendung der `__settile...`- und CGB-Kachelfunktionen einbinden. |
| `cgb_palette.h` / `cgb_palette.c` | Höhere Abstraktion für BG-/OBJ-Paletten auf CGB. | Header einbinden und Quelle mitkompilieren. |
| `scroll.h` / `scroll.c` | Scroll- und Split-Tabellenfunktionen auf Basis der Intrinsics. | Für `Scroll_*`-Aufrufe einbinden. |
| `raster.h` / `raster.c` | Aufbau von Scrollbändern und strukturierten X-Verzerrungsprofilen je Scanline. | Für `Raster_*` zusammen mit `scroll.c` kompilieren. |
| `camera.h` / `camera.c` | Q8.8-Kamera auf Basis von `scroll.*`, globale Kamerahelfer und Welt-/Bildschirmumrechnung. | Header einbinden und Quelle mitkompilieren. |
| `audio.h` / `audio.c` | GB-Audiotreiber für Musik, Effekte, Panorama, Wellenformen und Überblendungen; 68 Notenindizes bis 67 (`G6`). | Für Projekte mit Audio einbinden. |
| `audio_vblank.h` / `audio_vblank.c` | VBlank-IRQ-Musiktreiber mit demselben Indexbereich, WRAM-Warteschlange für 16 Ereignisse und optionalem Callback pro Bild. | Direkt adressierte Musik in Bank 0 ablegen oder die Warteschlange aus bankiertem Code nachfüllen; Vektor 0x0040 mit `scripts/patch_gb_vblank_irq.ps1` verbinden. |
| `link.h` / `link.c` | Serielle Byteübertragung und kooperative logische `Link4_*`-Funktionen. | Für Spiele mit Verbindungskabel einbinden. |
| `link_packet.c` | Optionale Paketschicht auf `link.c` mit getrennten `Link4_*`-Postfächern je Teilnehmer. | Zusammen mit `link.c` kompilieren, wenn Paketfunktionen benötigt werden. |
| `link_dmg07.h` / `link_dmg07.c` | Polling-Treiber mit externem Takt für den physischen Nintendo-DMG-07-Vierspieleradapter. | Mit `link_hwregs_gb.c` kompilieren; getrennt von der logischen `Link4_*`-API. |
| `rpg.h` | Gemeinsame Deklarationen für RPG-, Adventure- und Strategiespielfunktionen sowie maschinennahe Intrinsics. | Bei Verwendung dieser Funktionsfamilie einbinden. |
| `rng.c` | `rng8`, `rng16`, `rand_range`, `weighted_choice`, `rng_seed`, `rng_next8`, `rng_next16`, `rng_range` und `rng_chance`. | Für Zufallsfunktionen aus `rpg.h` kompilieren. |
| `flags.c` | Bitmenge mit 2048 Flags und Speicherung von Aufgabenzuständen. | Für Flag- und Quest-Funktionen aus `rpg.h` kompilieren. |
| `rle.c` | Einfache RLE-Dekodierung von `[count][value]` aus RAM oder bankiertem ROM. | Für `rle_decode*` aus `rpg.h` kompilieren. |
| `text.c` | Kacheltextfenster, Seitenwechsel, Auswahl, XY-Text, Zahlenausgabe sowie Lösch- und Fensterhelfer. | Für Textfunktionen aus `rpg.h` kompilieren. |
| `menu.c` | Vertikale Menüs, kleines Inventarmenü und nicht blockierende Menüzustände. | Für Menüfunktionen aus `rpg.h` kompilieren. |
| `script.c` | Kleiner Bytecode-Interpreter für RPG-/Adventure-Abläufe. | Für Skriptfunktionen aus `rpg.h` kompilieren. |
| `map.c` | Gepackte Karten mit Kollisionen, Auslösern, Kamera und optionalen 16 × 16-Metakacheln. | Für Kartenfunktionen aus `rpg.h` kompilieren. |
| `save.c` | SRAM-Speichern, Laden, Prüfen und Löschen nach dem MBC5-Schema, mit Header, Version, Länge und Prüfsumme. | Für Speicherfunktionen aus `rpg.h` kompilieren. |
| `slg_unit.c` | Bewegungs- und Angriffsreichweiten taktischer Einheiten. | Für Einheitenfunktionen aus `rpg.h` kompilieren. |
| `slg_path.c` | Breitensuche für Wege und kostenbasierte Erreichbarkeitsberechnung. | Für taktische Wegsuche aus `rpg.h` kompilieren. |
| `slg.h` / `slg_board.c` | Allgemeine Brett-, Zuglisten- und Rücknahmestapel-Funktionen für Brettspiele oder Taktiksysteme. | Header und Quelle einbinden; spielspezifische Bewertungen getrennt halten. |

### Hilfseinheiten

| Dateien | Aufgabe | Hinweise |
| --- | --- | --- |
| `audio_hwregs_gb.c` | Minimale Deklarationen der APU- und Wave-RAM-Register. | Nur verwenden, wenn keine andere Einheit diese Register bereits deklariert. |
| `link_hwregs_gb.c` | Minimale Deklarationen von `SB`, `SC`, `IF` und `IE`. | Nur verwenden, wenn die seriellen Register nicht bereits an anderer Stelle deklariert sind. |
| `cgb_tile.c` | Bewusst leere Übersetzungseinheit der CGB-Kachelfunktionen. | Die öffentliche Schnittstelle steht in `cgb_tile.h`; die C-Datei muss nicht mitkompiliert werden. |
| `math.c` | Sinustabelle im ROM (`MATH_SIN`). | Noch keine als stabil dokumentierte öffentliche API; vorerst als projektspezifische Dateneinheit behandeln. |

### Referenzdokumente

| Dateien | Inhalt |
| --- | --- |
| `README.md` | Englische Übersicht und Build-Hinweise. |
| `wire3d_guide_ja.md` | Japanischer Schnellstart für den Drahtgitterrenderer. |
| `dmg3d_guide_ja.md` | Japanischer Schnellstart für den gepufferten DMG-Drahtgitterrenderer. |
| `wire3d_cgb_guide.md` | Schnellstart für den CGB-Farbrenderer. |
| `physics_guide.html` | Englische Physikanleitung. |
| `physics_guide_ja.html` | Japanische Physikanleitung. |

## Bibliotheken kompilieren

Die Befehle zeigen, wie Anwendungsquellen und Bibliotheksmodule gemeinsam kompiliert werden. Stellen Sie die im jeweiligen Befehl genannten Anwendungsdateien bereit. Für die mitgelieferten Einsteigerprogramme verwenden Sie `../examples/build.ps1` und das HTML-Handbuch. Der Kurzaufruf `kitaqgb` setzt voraus, dass die ausführbare Datei im PATH liegt.

Kompilieren Sie Ihren Spielcode gemeinsam mit den benötigten Bibliotheksdateien:

```powershell
kitaqgb hwregs.c lib/audio.c main.c lib/physics2d.c lib/physics2d_circle.c lib/physics3d.c lib/cgb_palette.c lib/scroll.c lib/camera.c -I lib -o game.gb --profile=dev
```

Für ein Drahtgitterprojekt mit dem 96-Zeilen-Profil:

```powershell
.\kitaqgb.exe lib/wire3d.c examples/wire3d_minimal.c -I lib -o examples/wire3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

Das mitgelieferte `examples/wire3d_minimal.c` initialisiert sämtliche Modellfelder und dreht einen Würfel im 128 × 96-Ausschnitt. Es benötigt keine externen Grafik- oder Schriftdateien.

Für 128 × 120 können Sie den kompatiblen Einstiegspunkt `dmg3d.*` verwenden:

```powershell
.\kitaqgb.exe lib/dmg3d.c examples/dmg3d_minimal.c -I lib -o examples/dmg3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

`DMG3D_Init()` richtet eine sichtbare Drahtgitterfläche von 128 × 120 mit WRAM-Puffer ab D000 und Kachelübertragung ab 0x8900 ein. `DMG3D_BeginFrame()` setzt nur den Verdeckungszustand zurück; die Übertragungen verbrauchen und löschen die Pixel. `DMG3D_EndFrame()` wartet auf VBlank und prüft dann STAT während der Kopie, die über VBlank hinausreichen kann. Das mitgelieferte `examples/dmg3d_minimal.c` zeichnet pro Bild ein Kreuz mit aktivierter Teilübertragung neu. Die getrennte Hilfsübertragung teilt ihren Quellspeicher mit dem Hauptpuffer.

CGB-exklusive Farbprojekte verwenden `wire3d_cgb.*`:

```powershell
kitaqgb lib/wire3d_cgb.c examples/wire3d_cgb_color_demo.c -I lib -o examples/wire3d_cgb_color_demo.gbc --profile=dev --stack-bank=fixed --rst-disable --cgb=cgb_only --rom-title=CGBWIRE3D
```

`Wire3DCGB_Init()` schaltet die CGB auf doppelte Taktrate, richtet eine 128 × 96-BG-Fläche mit 2bpp ein und installiert die Standardpalette mit vier Einträgen. Beide Paare von Bildfunktionen übertragen den 3072-Byte-Puffer bei `0xD300–0xDEFF` per HBlank-DMA in die inaktive VRAM-Kachelbank und zeigen diese während VBlank an. LCDC wird dabei nicht für jedes Bild umgeschaltet. Das `Fast`-Paar lässt die übliche BG-Warteschlangenprüfung am Bildende aus. Farben wählen Sie mit `Wire3DCGB_SetPaletteRGB15()`, `Wire3DCGB_SetLineColor()` beziehungsweise `Wire3DCGB_Draw*Color()`.

Für aus CAD-Daten erzeugte richtungsabhängige Detailstufen nimmt `Wire3DCGB_DrawMaskedModel2D()` vorprojizierte vorzeichenbehaftete Eckpunktversätze und eine gepackte Maske sichtbarer Kanten entgegen. Kantenschleife und Assembler-Rasterizer liegen in Rendererbank 4; dadurch fällt nicht für jede Linie ein bankübergreifender Aufruf an.

Bei Teilübertragungen mit OAM-Schattenpuffer können Sie nach Eintritt in VBlank zuerst `sprite_flush_oam()` und dann `Wire3DCGB_EndFrameSparseNow()` aufrufen. Das vermeidet ein anfängliches Warten auf den nächsten VBlank. DMA und Präsentation können aber je nach Änderungsbereich und aktueller Scanline weiterhin warten; die Aufrufe garantieren keinen Abschluss im selben VBlank.

Für CGB-Linien gelten die Farben 1, 2 und 3. Normale 128 × 96-Linien kombinieren Farbbits, sodass Farbe 1 und 2 zusammen Farbe 3 ergeben. Farbe 0 löscht keine Linie. Verwenden Sie einen Bildreset oder die Löschfunktionen. `Wire3DCGB_DrawLine2D` und Modelldarstellung erfassen keine Bereiche für Teilübertragungen: Verwenden Sie dafür `Wire3DCGB_DrawLineClipped2D` oder nehmen Sie mit `Wire3DCGB_InvalidateFrameHistory` die gesamte Zeichenfläche in die nächste Teilübertragung auf.

Der Vollbildmodus mit 160 × 144 reserviert höchstens 127 Kacheln pro Bild. Fehlgeschlagene Reservierung oder ungültige Koordinaten im schnellen Linienpfad setzen das über `Wire3DCGB_GetFullScreenOverflow()` abfragbare Fehlerflag. Weitere Pixel werden bis zum nächsten Bildreset nicht geschrieben. Eckpunkte müssen innerhalb der gewählten Fläche liegen; der rechte Rand der Dreiecksmaske endet bei X=127 beziehungsweise X=159. Beachten Sie insbesondere bei Vollbild und FastMap die dokumentierte WRAM-Bankzuordnung. Der [CGB-Grenztest](../tests/library/wire3d_cgb_mask_bounds.c) ist ein vollständiges Prüfprogramm für beide Größen.

Build-Vorlagen für RPG-, Adventure- und Strategiespielfunktionen:

```powershell
kitaqgb examples/example_rpg_text.c lib/text.c lib/menu.c -I lib -o text.gb --profile=dev
kitaqgb examples/example_adv_script.c lib/text.c lib/flags.c lib/script.c -I lib -o script.gb --profile=dev
kitaqgb examples/example_slg_cursor.c lib/map.c lib/slg_unit.c lib/slg_path.c -I lib -o slg.gb --profile=dev
```

Vorlage für einen Basistest der Standardlaufzeit:

```powershell
kitaqgb lib/text.c lib/menu.c lib/map.c lib/scroll.c lib/camera.c lib/rng.c lib/save.c lib/system.c lib/input.c lib/vram.c lib/sprite.c lib/fixed.c lib/scene.c lib/entity.c lib/bank.c lib/asset.c lib/debug.c lib/chain.c lib/physics2d.c lib/slg_board.c examples/standard_library_smoke.c -I lib -o examples/standard_library_smoke.gb --profile=dev --rom-title=STDLIBSMK --no-disasm
```

Für serielle Projekte muss die Registereinheit vor dem Verbindungskern kompiliert werden:

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c main.c -I lib -o game.gb --profile=dev
```

Kooperative, vom Host gesteuerte logische Vierspielerprojekte verwenden dieselben Dateien. Der Host ruft `Link4_InitHost(slot_count)` auf und wählt den Teilnehmer mit `Link4_SelectPeer()` oder `Link4_SendPacketTo()`. Teilnehmer verwenden `Link4_InitPeer(local_slot, slot_count)` und kommunizieren mit Host-Slot `0`.

Die historischen Teilnehmerbeispiele lassen sich mit den jeweiligen Slot-Einstiegspunkten bauen, sofern deren Quellen vorhanden sind:

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot1.c -I lib -o peer1.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot2.c -I lib -o peer2.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot3.c -I lib -o peer3.gb --profile=dev
```

Für den physischen DMG-07-Adapter verwenden Sie stattdessen den eigenen Polling-Treiber:

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link_dmg07.c main.c -I lib -o dmg07.gb --profile=dev
```

Rufen Sie `LinkDmg07_Poll()` fortlaufend auf; die Adapterbytes liegen zeitlich viel dichter beieinander als ein Bildwechsel. `LinkDmg07_TickFrame()` wird einmal pro VBlank für die sättigenden Ruhe- und Verbindungszeitgeber aufgerufen. Der Treiber aktiviert immer den externen Takt mit `SC=$80`, beantwortet Abfragen mit `88 88 RATE 01` und erlaubt nur dem physischen Spieler 1, per `AA AA AA AA` eine Übertragung anzufordern. Nachdem alle Konsolen `CC CC CC CC` erkannt haben, enthält jedes Vier-Byte-Paket je ein Byte pro physischem Slot. Die Daten werden erst im Paket nach ihrer Einreichung verteilt. Deshalb verwirft der Treiber das erste undefinierte Paket und stellt Sende- und Empfangssequenzwerte bereit.

`LinkDmg07_RequestRestart()` wartet auf die nächste Paketgrenze, sendet ausgerichtet `FF FF FF FF` und stoppt nach dem vollständigen All-FF-Signal des Adapters. Ein Ruhezeitüberschritt während der Übertragung plant diesen Neustart automatisch ein, ohne die aktuelle Position im Vier-Byte-Paket zu verwerfen. Wenn der Adaptertakt zurückkehrt, kann so erst das Paket abgeschlossen und dann die Wiederherstellung sicher begonnen werden.

Binden Sie im Spielcode die benötigten Header ein:

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

## Hinweise

Die letzten acht Audionotenindizes verwenden derzeit die Frequenzen der vorigen Oktave erneut. 68 Indizes bedeuten deshalb nicht 68 verschiedene Tonhöhen.

- `inv_mass_q8 == 0` kennzeichnet einen statischen Körper.
- `Wire3D_Init()` belegt eine 128 × 96-BG-Fläche, WRAM-Puffer ab `0xD000` und Kacheldaten ab `0x8900`. Kompilieren Sie mit `--stack-bank=fixed`.
- `Wire3D_BeginFrame()` leert den WRAM-Puffer; `Wire3D_EndFrame()` wartet auf VBlank und kopiert anschließend in STAT-gesteuerten Blöcken nach VRAM.
- Wire3D-Winkel verwenden 16 Schritte pro Umdrehung; der Modellpfad unterstützt bis zu `WIRE3D_MODEL_VERTEX_LIMIT` Eckpunkte.
- `Wire3D_DrawScene()` zeichnet nahe Objekte zuerst und sammelt Flächenmasken, um weiter entfernte Linien konservativ zu unterdrücken.
- `Wire3DCGB_Init()` ist CGB-exklusiv und schaltet per KEY1/STOP auf doppelte Taktrate. Verwenden Sie `--cgb=cgb_only` und kombinieren Sie `wire3d_cgb.*` nicht mit `wire3d.*` in derselben ROM.
- Standard- und `Fast`-Bildfunktionen lassen das alte Bild sichtbar, bis die inaktive VRAM-Bank vollständig übertragen ist. `Fast` eignet sich, wenn keine vorgemerkten BG-Kachelschreibzugriffe benötigt werden.
- Die Datei mit den Deklarationen von `NR10..NR52` und `WAVE0..WAVE15` muss vor `lib/audio.c` kompiliert werden.
- Dafür steht `lib/audio_hwregs_gb.c` bereit. Verwenden Sie es nicht zusätzlich zu einer anderen Einheit mit denselben Registern.
- `cgb_tile.h` stellt Compiler-Intrinsics direkt bereit. `lib/cgb_tile.c` ist nur ein Platzhalter und kann entfallen.
- Die öffentliche Schnittstelle von `cgb_palette.h` verwendet das Präfix `cgb_*`.
- Rufen Sie `Audio_SetMusicEnabled()` beziehungsweise `Audio_SetSfxEnabled()` bei Änderungen in Menüs oder Einstellungen auf.
- `Audio_PlaySFX()` übernimmt die gerade sichtbare ROM-Bank. Wenn die Datenbank bekannt ist, verwenden Sie `Audio_PlaySFXBanked(bank, sfx, priority)`.
- Die Musikbefehle `AUDIO_CMD_NOTE` und `AUDIO_CMD_SET_INST` behalten die Reihenfolge `0=CH1`, `1=CH2`, `2=CH4`, `3=CH3` bei.
- `Audio_LoadCustomWave()` erhält 16 Bytes mit 32 gepackten 4-Bit-Abtastwerten für eine eigene CH3-Wellenform.
- `Audio_FadeToMasterVolume()` schreitet nur bei `Audio_Update()` fort. Aktualisieren Sie während einer Überblendung in jedem Bild.
- `audio_vblank.c` besitzt das VBlank-Vektorsymbol `__kq_vblank_vector`. Ein Musikereignis hat fünf Bytes: `delay, ch2_note, ch1_note, ch3_note, ch4_noise_param`. Verwenden Sie `AUDIO_VBLANK_REST`, `AUDIO_VBLANK_LOOP` und `AUDIO_VBLANK_END`.
- Direkt adressierte IRQ-Musik muss in der festen Bank liegen. Im Warteschlangenmodus kann bankierter Spielcode nachfüllen; eine fertige Nachfüllroutine ist nicht enthalten. Nach dem Linken mit `lib/audio_vblank.c` verbindet `scripts/patch_gb_vblank_irq.ps1 <rom> <map>` den Vektor 0x0040 mit der ISR und erneuert die ROM-Prüfsummen.
- Kombinieren Sie `audio_vblank.c` nicht mit einer anderen Einheit, die Vektor 0x0040 beansprucht, ohne einen gemeinsamen Interrupt-Verteiler einzubauen.
- `Scroll_SplitCommit()` aktiviert die IE-Bits `0x01 | 0x02` und verwendet die VBlank-/STAT-Handler des Compilers.
- Die Split-Funktionen reservieren dabei 0x0040 und 0x0048. Kombinieren Sie sie vorerst nicht mit eigenen getrennten VBlank-/STAT-Stubs.
- Die Datei mit `SB`, `SC`, `IF` und `IE` muss vor `lib/link.c` / `lib/link_packet.c` beziehungsweise `lib/link_dmg07.c` kompiliert werden.
- Dafür steht `lib/link_hwregs_gb.c` bereit. Vermeiden Sie doppelte Deklarationen derselben Register.
- Die Verbindungsbibliothek belegt Vektor 0x0058 nicht selbst. Im Interrupt-Modus muss Ihr eigener Handler oder Verteiler `Link_OnSerialIRQ()` aufrufen.
- Die Paketschicht hält bewusst nur ein Paket bereit und wird am besten aus einer am Bildwechsel ausgerichteten Hauptschleife bedient.
- `Link4_*` bildet einen kooperativen, vom Host ausgewählten Vierspieleradapter ab. Auf der Leitung ist jeweils nur ein Teilnehmer aktiv; der Host muss diese gezielt wechseln.
- `Link4_TryReadByteFrom()` und `Link4_HasPacketFrom()` erlauben die Abfrage getrennter Teilnehmerpostfächer ohne Verlust der Absenderzuordnung.
- `Link_ReadPacket()` bleibt die ältere Sicht auf das zuletzt empfangene Paket. Für vier Spieler verwenden Sie `Link4_ReadPacketFrom()`.
- `Link4_*` bildet weder die Elektrik noch das Protokoll des Nintendo DMG-07 ab. Für das reale Zubehör verwenden Sie `link_dmg07.c`; kompilieren Sie es nicht zusammen mit `link.c` in derselben ROM.
- DMG-07 `GetConnectedMask()` verwendet Bits 0..3 für die physischen Spieler 1..4. Während der Übertragung bleibt das Ergebnis der letzten Abfrage bestehen; die Teilnehmerliste lässt sich nur in der Abfragephase erneuern.
- Ohne Adaptertakt kann ein geplanter Neustart nicht fortschreiten. Wiederherstellungsdaten werden verworfen und nicht als Nutzdaten an den Sequenzer weitergegeben. Wurde das Zubehör aus- und eingeschaltet und dadurch in eine andere Phase versetzt, initialisieren Sie Treiber und Sitzung ausdrücklich neu.
- Die Physikbibliotheken behandeln lineare Position und Geschwindigkeit, keine Rotationsdynamik.
- Halten Sie die Zahl aktiver Körper auf GB-Hardware klein, beispielsweise bei 8 bis 24.
- Stimmen Sie Schwerkraft, Höchstgeschwindigkeit und Zahl der Löserdurchläufe pro Welt auf das gewünschte Spielverhalten ab.
- Für billardähnliche Spiele eignet sich `physics2d_circle.*` besser als die AABB-Bibliothek.
- Ergänzen Sie für den derzeitigen Aufbau keine getrennten Bibliotheken `random`, `collision`, `ui`, `tilemap`, `dialog`, `board_game` oder `simple_physics`. Verwenden Sie dafür `rng`, `physics2d`, `text`/`menu`, `map`, `script`, `slg` beziehungsweise `physics2d`.
- `scene.c` und `entity.c` vermeiden Funktionszeigerargumente in Zeigerbreite: Der aktuelle KITAQGB-Aufrufpfad ist für Aufrufe ohne Argumente oder mit einer Byte-ID am zuverlässigsten.
