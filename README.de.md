# KITAQGB

[English](README.md#english) | [日本語](README.md#japanese) | **Deutsch**

**[Compiler-Handbuch](https://bartaro.github.io/kitaq-docs/de/kitaqgb.html)** · **[Bibliothekshandbuch](https://bartaro.github.io/kitaq-docs/de/gb-library.html)**

## Herkunft des Namens

KITAQGB entstand als Fork von NORCAL, dem NES-C-Compiler im Umfeld von Zachtronics. Der Copyright-Vermerk des NORCAL-Autors Keith Holman bleibt erhalten.

NORCAL ist nach Nordkalifornien (Northern California) benannt. Diese geografische Namensgebung inspirierte den Autor dazu, KITAQGB nach Kitakyushu zu benennen, der Stadt, in der er geboren wurde und aufgewachsen ist. KITAQ + GB verbindet Game Boy mit **北九 (キタキュー, Kitakyū)**, dem Kurznamen von Kitakyushu in der japanischen Präfektur Fukuoka. KITAQ wird wie das japanische „キタキュー“ ausgesprochen. Als englische Aussprachehilfe dient **kee-tah-KYOO**, in Lautschrift **/ˌkiːtɑːˈkjuː/**. Das abschließende Q klingt wie der englische Buchstabenname Q. KITAQGB lautet auf Englisch **kee-tah-KYOO jee bee**; G und B werden einzeln ausgesprochen.

Der Name KITAQGB hat zwei Bedeutungen. **Kernel-Informed Toolchain for AI-Quality Game Boy Development** steht für das Ziel einer Toolchain, die die Zielhardware versteht und sowohl Menschen beim Programmieren als auch generative KI unterstützt.

Die zweite Bedeutung lautet **Kids' Imagination Transformed into Actual Quests in Game Boy Forests**: ein Werkzeug, das die Fantasie von Kindern in echte Abenteuer in den Wäldern des Game Boy verwandelt. Darin steckt der kreative Wunsch, aus kleinen Ideen, Kritzeleien und mit KI erstellten Prototypen Abenteuer zu machen, die man tatsächlich spielen kann.

## Projektstatus: öffentliche Vorabversion

KITAQGB und KOKURA sind derzeit als öffentlich zugängliche Entwicklungswerkzeuge in einer Vorabversion verfügbar.

Sie eignen sich für Experimente, Beispielprojekte, KI-gestützte Spieleentwicklung, Compilerforschung, emulatorgestützte Fehlersuche und die Prüfung von Arbeitsabläufen. Die Projekte werden jedoch aktiv weiterentwickelt. APIs, Kommandozeilenoptionen, Ausgabeformate, Diagnosen und Verhalten können sich zwischen Versionen ändern.

Vorabversionen können Fehler, unvollständige Funktionen oder inkompatible Änderungen enthalten. Prüfen Sie erzeugten Code, Emulatorverhalten, Zeitdiagnosen und Berichte sorgfältig, bevor Sie sie produktiv einsetzen oder Ergebnisse öffentlich veröffentlichen.

**Kernel-Informed Toolchain for AI-Quality Game Boy Development**

KITAQGB ist eine quelloffene C-Toolchain für selbst entwickelte Software auf Game Boy und Game Boy Color. Sie unterstützt KI-gestützte Entwicklung mit gut auswertbaren Diagnosen: Schreiben Sie kleine C-Programme, kompilieren Sie diese zu `.gb`- oder `.gbc`-ROMs und nutzen Sie Rückmeldungen aus dem Emulator für die nächste Überarbeitung des Spiels.

KITAQGB ist unabhängig von Nintendo und wird von Nintendo weder unterstützt noch gesponsert oder anerkannt. Game Boy und Game Boy Color sind Marken von Nintendo.

## Was KITAQGB bietet

KITAQGB besteht aus einem C-Compiler und Hilfsbibliotheken für Homebrew-Entwicklung auf Game-Boy-Hardware. Es baut auf NORCAL auf und erweitert diese Grundlage zu einem umfassenderen Werkzeugablauf für moderne, KI-gestützte Spieleentwicklung.

Die Schwerpunkte sind:

- Übersetzung von C in Game-Boy-ROMs;
- Homebrew für Game Boy und Game Boy Color;
- KI-taugliche Diagnosen und reproduzierbare Build-Berichte;
- ROM- und Header-Erzeugung sowie maschinennahe Codeerzeugung für LR35902-Zielsysteme;
- Bibliotheken für CGB-Paletten, Kacheln, Scrollen, Kamera, Audio, Kommunikation sowie RPG-, Adventure- und Strategiespielfunktionen;
- Zusammenarbeit mit KOKURA CLI für Ausführungstests, Ablaufprotokolle und Fehlersuche.

KITAQGB enthält **keine kommerziellen ROMs, Nintendo-BIOS-Dateien, Nintendo-SDK-Dateien, proprietären Ressourcen oder offiziellen Nintendo-Entwicklungsmaterialien**.

## Zielplattformen

KITAQGB erzeugt Homebrew-ROMs für Game-Boy-kompatible, Game-Boy-Color-kompatible und gezielt CGB-exklusive Software. Typische Ausgabedateien sind:

```text
*.gb
*.gbc
```

Testen Sie erzeugte ROMs im Emulator und, soweit möglich, auf realer Hardware oder mit einer geeigneten Flash-Cartridge. Besonders bei Zeitverhalten, Interrupts, VRAM-/OAM-Zugriffen, Audio, Verbindungskabeln und Bankumschaltung sind die Hardwareanforderungen genau zu beachten.

## Aufbau des Repositorys

Die Compilerquellen liegen im gleichnamigen Unterordner `kitaqgb/`. Die fertige Release-Version und ihre Laufzeitkonfiguration befinden sich im Stammverzeichnis; Bibliotheken und Beispiele haben eigene Ordner.

```text
kitaqgb/                  # Repository root
├─ kitaqgb/               # Compiler build sources
│  ├─ *.cs
│  ├─ app.config
│  └─ kitaqgb.csproj
├─ kitaqgb.exe            # Prebuilt Release compiler
├─ kitaqgb.exe.config     # .NET Framework runtime configuration
├─ lib/                # C support libraries
├─ examples/           # Tutorial programs and original font
├─ scripts/build.ps1   # Rebuild the Release executable
├─ LICENSE
└─ LICENSE.ja
```

Zum Ausführen des mitgelieferten Compilers benötigen Sie Windows mit .NET Framework 4.8. Laden Sie das Repository als ZIP herunter, damit Programm, Konfiguration, Bibliotheken und Lizenzhinweise zusammenbleiben. Für einen eigenen Build benötigen Sie zusätzlich das .NET Framework 4.8 Developer Pack und Visual Studio Build Tools. Im Stammverzeichnis:

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
.\examples\build.ps1
```

Ein Release-Build kopiert Programm und Konfiguration ins Stammverzeichnis. Debug-Builds verbleiben unter `kitaqgb/bin/Debug` und überschreiben die bereitgestellte Release-Version nicht. Build-Caches und PDB-Dateien werden nicht mitgeliefert. Eingabedaten und SHA-256-Prüfsummen stehen im [Build-Nachweis](BINARY_BUILD.json).

## Voraussetzungen

Der vorgesehene Build-Weg verwendet Windows, die Referenzdateien für .NET Framework 4.8 und Visual Studio beziehungsweise Visual Studio Build Tools mit MSBuild. Die Projektdatei ist ein klassisches C#-Projekt für `.NET Framework v4.8`.

Andere Betriebssysteme können mit Mono/MSBuild funktionieren, abhängig von den installierten Referenzassemblies. Der vorrangig unterstützte Weg bleibt Windows mit MSBuild.

## KITAQGB kompilieren

Führen Sie im Stammverzeichnis aus:

```powershell
msbuild kitaqgb\kitaqgb.csproj /p:Configuration=Release
```

Nach einem erfolgreichen Build kopiert das Projekt die ausführbare Datei ins Stammverzeichnis:

```text
kitaqgb.exe
```

Prüfen Sie anschließend die Kommandozeilenhilfe:

```powershell
.\kitaqgb.exe --help
```

## Schnellstart

Erzeugen Sie ein kleines Vorlagenprojekt:

```powershell
.\kitaqgb.exe template hello.c --overwrite
```

Kompilieren Sie es:

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=dev --fast-build --cache
```

Für einen Release-orientierten Build:

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=release --cache
```

Starten Sie die ROM in einem Game-Boy-/Game-Boy-Color-Emulator Ihrer Wahl oder in KOKURA CLI, wenn Sie dessen Debugging- und Beobachtungsfunktionen verwenden möchten.

## Mitgelieferte Bibliotheken verwenden

`lib/` enthält wiederverwendbaren C-Code. Kompilieren Sie die benötigten Bibliotheksdateien zusammen mit Ihrem Spiel und geben Sie mit `-I lib` das Headerverzeichnis an.

Beispiel mit Audio, Paletten, Scrollen, Kamera und Physik. Die folgenden mehrzeiligen Befehle verwenden die Fortsetzungszeichen von **cmd.exe**:

```cmd
.\kitaqgb.exe main.c ^
  lib\audio_hwregs_gb.c lib\audio.c ^
  lib\cgb_palette.c lib\scroll.c lib\camera.c ^
  lib\physics2d.c lib\physics2d_circle.c lib\physics3d.c ^
  -I lib -o game.gb --profile=dev --fast-build --cache
```

Beispiel für ein Projekt mit serieller Verbindung:

```cmd
.\kitaqgb.exe lib\link_hwregs_gb.c lib\link.c lib\link_packet.c main.c ^
  -I lib -o link_game.gb --profile=dev --fast-build --cache
```

Beispiel für RPG-, Adventure- oder Strategiespielfunktionen:

```cmd
.\kitaqgb.exe main.c lib\text.c lib\menu.c lib\flags.c lib\script.c lib\map.c lib\save.c ^
  -I lib -o rpg.gb --profile=dev --fast-build --cache
```

Die aktuelle Einteilung und Hinweise zu den Bibliotheken stehen in [lib/README.de.md](lib/README.de.md).

## Häufig verwendete Optionen

```text
-o <file>                  Ausgabepfad der ROM
-I <dir>                   Include-Verzeichnis
--profile=dev              Entwicklungsprofil
--profile=release          Release-Profil
--fast-build / --fast      Schneller Build-Weg für die Entwicklung
--cache                    Build-Cache einschalten
--no-cache                 Build-Cache ausschalten
--disasm                   Disassemblierung ausgeben
--no-disasm                Keine Disassemblierung ausgeben
--diag-json <file>         Diagnosen als JSON schreiben
--machine-readable         Maschinenlesbare Ausgabe bevorzugen
--deps-out <file>          Abhängigkeitsinformationen ausgeben
--debug-output <dir>       Debug- und Hilfsdateien ablegen
--strict                   Ausgewählte Warnungen als Fehler behandeln
--permissive               Ausgewählte Diagnosen lockern
--stack-bank=fixed|wramx1  Stack-Bankmodell wählen
--stack-top=<addr>         Oberste Stack-Adresse wählen
--stack-reserve=<bytes>    Stack-Bereich reservieren
```

Die genaue Optionsliste Ihrer Version erhalten Sie mit:

```powershell
.\kitaqgb.exe --help
```

## Zusammenarbeit mit KOKURA CLI

KOKURA CLI ergänzt KITAQGB als Emulator und Debugger. Ein typischer Ablauf ist:

1. C-Spielcode schreiben oder erzeugen lassen.
2. Mit KITAQGB kompilieren.
3. Die erzeugte ROM in KOKURA CLI ausführen.
4. Diagnosen, Ablaufprotokolle, Symbole, Zeitmessungen und Emulatorberichte aufzeichnen.
5. Die Ergebnisse für die nächste Codeänderung oder Fehlersuche verwenden.

Bei KI-gestützter Entwicklung lassen sich Compilerfehler, Emulatorberichte und Laufzeitprotokolle so in klar eingegrenzte Korrekturaufgaben überführen.

## Entwicklungsgrundsätze

KITAQGB ist kein universeller moderner C-Compiler. Die Toolchain ist auf eine kleine 8-Bit-Spielplattform mit Speicherbanken und genauen Zeitanforderungen zugeschnitten.

Wichtig sind vorhersehbarer Maschinencode, verständliche Diagnosen, kleine reproduzierbare Beispiele, für Menschen und KI lesbare Berichte, maschinennahe Kontrolle bei Bedarf und einfach nutzbare Hilfsbibliotheken, wo dies möglich ist. Die Entwicklung soll zugänglicher werden, ohne die Hardware vollständig zu verbergen.

<!-- development-prompt:de:start -->
## Prompt zur Spieleentwicklung

Tragen Sie die Anforderungen ein und geben Sie den vollständigen Prompt an Ihre KI weiter. Er umfasst die Implementierung, Emulator-Tests, die Analyse mit SARAKURA und die erneute Prüfung nach Korrekturen.

[Das Praxisbeispiel im HTML-Handbuch lesen](https://bartaro.github.io/kitaq-docs/de/kitaqgb.html#loop-prompts)

<details>
<summary>Vollständigen Prompt anzeigen</summary>

### Spieleentwicklung mit KITAQGB, KOKURA und SARAKURA

Tragen Sie die Anforderungen ein und geben Sie dieses gesamte Dokument an die KI weiter. Die Befehle setzen voraus, dass die Repositories `kitaqgb`, `kitaqfc`, `kokura`, `kurosaki`, `sarakura` und `kitaq-docs` sowie das Projekt `game-gb` oder `game-fc` im selben übergeordneten Ordner liegen. Führen Sie die Befehle dort aus und passen Sie die Pfade an die tatsächliche Umgebung an.

#### Anforderungen

- Spieltitel: &lt;ausfüllen&gt;
- Genre und zentrale Spielmechanik: &lt;ausfüllen&gt;
- Steuerung sowie Erfolgs- und Misserfolgsbedingungen: &lt;ausfüllen&gt;
- Erforderliche Bildschirme, Level, Gegner und Gegenstände: &lt;ausfüllen&gt;
- Grafikstil, Musik und Geräusche: &lt;ausfüllen; bereitgestellte Dateien angeben&gt;
- Speichern, Kommunikation, Zusatzgeräte und weitere Anforderungen: &lt;ausfüllen oder keine&gt;
- Projektordner: &lt;ausfüllen&gt;
- Bedingungen für die Weitergabe: &lt;etwa eigener Code und eigene Medien, die unter MIT veröffentlicht werden können&gt;

- Zielgerät: &lt;ursprünglicher Game Boy / GB und CGB / nur CGB&gt;
- Leistungsziel: &lt;etwa 60 Aktualisierungen der Spiellogik pro Sekunde im normalen Spiel; akzeptables Verhalten in aufwendigen Szenen festlegen&gt;

#### Arbeitsauftrag

Setzen Sie das Spiel mit KITAQGB und seinen Bibliotheken um. Verwenden Sie KOKURA zum Ausführen und Debuggen und SARAKURA zum Auswerten der Diagnosen sowie zum Vergleich vor und nach einer Korrektur.

Wiederholen Sie diesen Ablauf, bis die Abnahmekriterien erfüllt sind: Spezifikation konkretisieren → kleine Änderung umsetzen → bauen → Eingaben ausführen und beobachten → Ursache untersuchen → korrigieren → unter gleichen Bedingungen erneut testen. Ein Plan, ausgegebener Quellcode oder ein erfolgreicher Compilerlauf allein schließen den Auftrag nicht ab.

##### Umgebung und Abnahmekriterien klären

1. Lesen Sie die Anweisungen im Arbeitsverzeichnis, READMEs, HTML-Handbücher sowie Header und Implementierungen der verwendeten Bibliotheken. Erfassen Sie die Pfade der Programme und ihre Versionen oder SHA-256-Werte. Prüfen Sie Befehle anhand der tatsächlichen `--help`-Ausgabe und APIs anhand des Quellcodes.
2. Legen Sie überprüfbare Kriterien für Eingaben, Bild, Ton, Spielverlauf und Aktualisierungsrate fest. Beispiele: START drücken und loslassen beginnt das Spiel; eine Kollision kostet ein Leben; die Pause schaltet die vorgesehenen Töne stumm, Fortsetzen nimmt die Wiedergabe wieder auf.
3. Fragen Sie nur bei wesentlichen Unklarheiten nach. Treffen Sie übliche, rückgängig zu machende Implementierungsentscheidungen selbstständig. Schwächen Sie Anforderungen und Abnahmekriterien nicht ab.
4. Führen Sie zunächst ein kleines mitgeliefertes Beispiel durch Compiler, Emulator und SARAKURA. Das prüft deren Zusammenspiel, nicht die Fertigstellung des beauftragten Spiels.

##### Eine kleine spielbare Fassung umsetzen

- Verwenden Sie den C-Dialekt von KITAQGB und `void main()`. Setzen Sie Desktop-C- oder GBDK-APIs nicht als verfügbar voraus. Binden Sie die benötigten `.c`-Implementierungen ein, nicht nur Deklarationen; prüfen Sie Initialisierung, Einheiten, Vorzeichen, Wertebereiche, Pufferlebensdauer und ROM-Bänke.
- Planen Sie VRAM/OAM-Aktualisierungen, VBlank, Interrupts, Stack, ROM/WRAM-Bänke sowie Grenzen für Tiles und Sprites. Gesamt- und Restkapazität der Übertragungswarteschlange sind etwas anderes als Kapazität und freier Platz des physischen VRAM.
- Ein DMG-Spiel darf nicht von CGB-exklusiven Funktionen abhängen. Bei Unterstützung beider Geräte sind beide Hardwaremodi zu prüfen.
- Verwenden Sie für Buchstaben, Ziffern und Symbole die bereitgestellte eigene Schrift aus `ascii.c` und prüfen Sie die Zuordnung von Zeichen zu Tiles.

- Verbinden Sie zunächst Start, Titelbild, steuerbare Spielfigur, Erfolg oder Misserfolg und Neustart. Erweitern Sie danach den Inhalt.
- Bewahren Sie bearbeitbare Originale von Grafik, Musik und Geräuschen sowie deren Erzeugungsschritte auf. Prüfen Sie, dass der Build tatsächlich die exportierten Daten verwendet.
- Schreiben Sie Codekommentare auf Englisch und Fortschrittsberichte auf Deutsch. Lassen Sie die Standardberichte von SARAKURA auf Englisch.

##### Jeden Build seiner Ausführung zuordnen

Trennen Sie Ausgaben nach Iteration, etwa mit `out/iter-001`. Protokollieren Sie Befehle, Rückgabecodes und Hashes von Quellcode, Medien, Werkzeugen, ROM und Metadaten. Führen Sie nach einem fehlgeschlagenen Build niemals eine alte ROM aus. Speicherbelegungspläne, Quellcodezuordnungen und Debuginformationen müssen aus demselben Build wie die ROM stammen.

Das folgende Beispiel ist eine grundlegende DMG-Prüfung. Stellen Sie `main.c` und alle erforderlichen Bibliotheksimplementierungen bereit und passen Sie Optionen und Eingabefolge an das Spiel an.

```powershell
$iteration = '.\game-gb\out\iter-001'
New-Item -ItemType Directory -Force $iteration | Out-Null

# Include all additional implementation units required by the game.
& '.\kitaqgb\kitaqgb.exe' '.\game-gb\src\main.c' `
  -I '.\kitaqgb\lib' -o "$iteration\game.gb" `
  --profile=dev --rst-disable --stack-bank=fixed --no-disasm `
  "--emit-ai-metadata=$iteration\build.json"
if ($LASTEXITCODE -ne 0) { throw 'Build failed; inspect the build log.' }

# This sequence presses START once, with released intervals on both sides.
& '.\kokura\kokura-cli.exe' "$iteration\game.gb" `
  --hardware dmg --run-frames 300 `
  --input-seq 'NONE:60;START:1;NONE:239' `
  --png "$iteration\frame.png" --record-wav "$iteration\audio.wav" `
  --dump-report "$iteration\run.json" `
  --emit-diagnostics "$iteration\events.jsonl"
if ($LASTEXITCODE -ne 0) { throw 'Emulator run failed; inspect the run log.' }

& '.\sarakura\sarakura.exe' gb analyze `
  --metadata "$iteration\build.json" --events "$iteration\events.jsonl" `
  --frames 300 --out "$iteration\analysis" --fail-on error
if ($LASTEXITCODE -ne 0) { throw 'Inspect the analysis report and fix the cause.' }
```


`--hardware dmg` wählt den ursprünglichen Game Boy. Stimmen Sie bei CGB oder Unterstützung beider Geräte den ROM-Header und den Hardwaremodus des Emulators aufeinander ab. Die Eingabefolge drückt START einmal zwischen Abschnitten mit losgelassenen Tasten. 300 Frames prüfen nicht das gesamte Spiel.

##### Bild, Ton, Zustand und Leistung prüfen

- Speichern Sie Eingabeszenarien mit getrenntem Drücken, Halten und Loslassen. Durchlaufen Sie alle vorgesehenen Wege: Start, Spielbeginn, Bewegung, Aktionen, Kollisionen, Scrolling, Levelwechsel, Spielende, Neustart, Pause und gegebenenfalls Speichern oder Kommunikation.
- Bewahren Sie PNGs relevanter Frames, Eingaben, Laufberichte, Diagnose-JSONL, WAVs und nötige Zustands- oder Speicherbeobachtungen auf. Prüfen Sie erreichte Frames und Abbruchgrund. Öffnen Sie die Bilder tatsächlich; ein einzelner Screenshot belegt weder Bewegung noch Eingabereaktion. Vergleichen Sie Zähler, Positionen und Zustandswechsel mit Sollwerten; prüfen Sie Bildschirmränder, Tile- und Attributgrenzen und Szenen mit vielen Sprites.
- Prüfen Sie Musik, Geräusche, gleichzeitige Wiedergabe, Aussetzer, Pause und Fortsetzen. Eine erzeugte WAV-Datei allein belegt keinen korrekten Klang. Ist Anhören nicht möglich, trennen Sie ausgeführte Wellenform- und Zahlenprüfungen von unbestätigten Höreigenschaften.
- Messen Sie aufwendige Szenen, Arbeit der Ziel-CPU, Spielaktualisierungen und Transfers; auf FC auch die NMI-Arbeit. Emulatordurchsatz auf dem Host ist weder Spielaktualisierungsrate noch Nachweis für reale Hardwaregeschwindigkeit. Fortsetzen mit `--allow-unimplemented` belegt keine Unterstützung der fehlenden Funktion.

##### Analysieren, korrigieren und erneut testen

- Übergeben Sie SARAKURA die Buildmetadaten der geprüften ROM und Diagnose-JSONL aus dem zugehörigen Lauf. Ein CPU-Trace oder gewöhnlicher Laufbericht ersetzt dies nicht. `--frames` legt Analysebedingungen fest; SARAKURA führt weder ROMs aus noch ändert es automatisch Quellcode.
- Lesen Sie `report.html`, `ai_diagnostics.json`, `repair_prompt.md` und `retest_plan.json`. Gleichen Sie Diagnosen mit Reproduktionsschritten, Bildern, Ton und Quellcode ab. Trennen Sie vermutete Quellstellen oder Ursachen von bestätigten Tatsachen und normale Warteschleifen von Hängern. Bewerten Sie Warnungen einzeln und dokumentieren Sie nicht unterstützte Ereignisse sowie Analysegrenzen. Verbergen Sie Warnungen nicht durch Filter und verkürzen Sie Tests nicht, um ein Bestehen zu erreichen.
- Reduzieren Sie Fehler auf minimale Reproduktionen, beheben Sie die Ursache und bauen Sie neu. Liegt der Fehler im Compiler oder Emulator, grenzen Sie ihn vom Spielcode ab und ergänzen Sie eine Regressionsprüfung für die Werkzeugkorrektur.
- Wiederholen Sie Tests mit gleichen Eingaben, Zufallsstartwerten, Hardware- und Videomodi, Mappern, Beobachtungsframes und Diagnoseeinstellungen. Nutzen Sie für jede ROM passende Metadaten; verwenden Sie nach Änderungen an Code oder RAM-Belegung nicht blind alte Speicherzustände weiter.

```powershell
& '.\sarakura\sarakura.exe' baseline-delta `
  --baseline '.\game-gb\out\iter-001\analysis' `
  --current '.\game-gb\out\iter-002\analysis' `
  --out '.\game-gb\out\delta.json' --markdown '.\game-gb\out\delta.md' `
  --fail-on-new error --fail-on-regression error --enforce
```


Nutzen Sie Diagnosedifferenzen gemeinsam mit den Abnahmekriterien für Steuerung, Grafik und Ton. Wiederholt sich derselbe Fehler, überprüfen Sie Belege und Hypothese, statt beliebige weitere Änderungen vorzunehmen.

##### Abschlusskriterien und Lieferumfang

Führen Sie alle Pflichtszenarien erneut mit der finalen ROM aus, die aus den gelieferten Quellen und Einstellungen gebaut wurde. Unverwundbarkeit, automatische Testeingaben oder ein anderer Mapper allein prüfen kein normales Spiel in der Endfassung. Liefern Sie eine Zuordnung von Anforderungen zu Tests, Gründe für verbleibende Warnungen und klare Angaben zu ungeprüften oder nicht unterstützten Punkten. Kennzeichnen Sie ausdrücklich, wenn nicht auf physischer Hardware getestet wurde.

Liefern Sie Quellcode, Kennungen von Werkzeugen und Bibliotheken, bearbeitbare Medien, reproduzierbare Build- und Testskripte, ROM, abschließende Nachweise und eine README mit Einrichtung, Steuerung und bekannten Grenzen. Fügen Sie bei Bedarf Replays und das Testprogramm hinzu. Veröffentlichen oder versenden Sie Dateien nur im ausdrücklich erlaubten Umfang. Entfernen Sie unnötige Zwischenbuilds und temporäre Traces nach der Prüfung, bewahren Sie jedoch Quellen, Medien, Endergebnisse und erforderliche Regressionsnachweise auf.

Verhindern Umgebung oder Berechtigungen eine Pflichtprüfung, nennen Sie die genauen Reproduktionsschritte und die nötige Maßnahme. Kennzeichnen Sie die Arbeit nicht als abgeschlossen.

</details>
<!-- development-prompt:de:end -->

## Marken und Unabhängigkeit

KITAQGB ist ein unabhängiges Open-Source-Projekt für Homebrew-Entwicklung. Es ist nicht mit Nintendo verbunden und wird von Nintendo weder unterstützt noch gesponsert oder anerkannt. Game Boy und Game Boy Color sind Marken von Nintendo.

Fügen Sie diesem Repository keine Nintendo-Logos, offiziellen Verpackungsgrafiken oder Schriften, BIOS-Dateien, kommerziellen ROM-Daten oder proprietären Spielressourcen hinzu, wenn Ihnen die erforderlichen Rechte fehlen.

## Lizenz

KITAQGB wird unter der MIT-Lizenz bereitgestellt. Es ist von NORCAL abgeleitet, dessen ursprünglicher Vermerk lautet:

```text
Copyright 2019 Keith Holman
```

Für Änderungen und Ergänzungen an KITAQGB gilt:

```text
Copyright (c) 2026 DAISUKE OBA
```

Der ursprüngliche NORCAL-Copyright-Vermerk und der MIT-Lizenztext müssen in Kopien oder wesentlichen Teilen der Software erhalten bleiben. Einzelheiten stehen in [LICENSE](LICENSE) und [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Beiträge

Beachten Sie vor dem Einreichen von Änderungen:

- Keine urheberrechtlich geschützten ROMs, BIOS-Dateien, extrahierten kommerziellen Ressourcen oder offiziellen SDK-Materialien hinzufügen.
- Erzeugte Build-Dateien wie `bin/`, `obj/`, `target/`, `dist/`, `*.exe`, `*.dll` und `*.pdb` nur aus einem konkreten Veröffentlichungsgrund in Quellcode-Commits aufnehmen.
- Für Compiler- und Codeerzeugungsfehler kleine, reproduzierbare Tests bevorzugen.
- Bei neuen Bibliotheken den benötigten Build-Befehl und erforderliche Hardwareregister-Deklarationen dokumentieren.
- Diagnosen so formulieren, dass Menschen und KI-Programmieragenten daraus konkrete Schritte ableiten können.

## Stand dieser Ausgabe

Dieses Repository ist für eine erste öffentliche Ausgabe von KITAQGB vorbereitet. Schnittstellen, Bibliotheken, Diagnosen und die Anbindung anderer Werkzeuge können sich mit der Weiterentwicklung ändern.

## Build und erster Aufruf

Verwenden Sie unter Windows das .NET Framework 4.8 Developer Pack und Visual Studio Build Tools. Führen Sie in einer Developer PowerShell aus:

```powershell
MSBuild.exe .\kitaqgb\kitaqgb.csproj /t:Build /p:Configuration=Release
.\kitaqgb.exe --help
.\examples\build.ps1
```

## Handbücher und Lizenzen

- [Deutsches Compiler-Handbuch](https://bartaro.github.io/kitaq-docs/de/kitaqgb.html) / [Deutsches Bibliothekshandbuch](https://bartaro.github.io/kitaq-docs/de/gb-library.html)
- [Japanisches Handbuch](https://bartaro.github.io/kitaq-docs/kitaqgb.html) / [Englisches Handbuch](https://bartaro.github.io/kitaq-docs/en/kitaqgb.html)
- [Handbuchquellen zum Offline-Lesen](https://github.com/bartaro/kitaq-docs)
- [Lizenz](LICENSE) / [Japanische Übersetzung zur Orientierung](LICENSE.ja)

Die Projektlizenz ersetzt keine Bedingungen Dritter für Schriften, Abhängigkeiten, Logos oder Marken. Bewahren Sie bei einer Weiterverteilung die beiliegenden Hinweise auf.
