# KITAQGB

[English](README.md#english) | [日本語](README.md#japanese) | **Deutsch**

**[Compiler-Handbuch](https://bartaro.github.io/kitaq-docs/de/kitaqgb.html)** · **[Bibliothekshandbuch](https://bartaro.github.io/kitaq-docs/de/gb-library.html)**

## Herkunft des Namens

KITAQGB entstand als Fork von NORCAL, dem NES-C-Compiler im Umfeld von Zachtronics. Der Copyright-Vermerk des NORCAL-Autors Keith Holman bleibt erhalten.

**NORCAL ist nach Nordkalifornien (Northern California) benannt.** Diese geografische Namensgebung inspirierte DAISUKE OBA, den Autor von KITAQGB, dazu, **Kitakyushu**, die Stadt, in der er geboren wurde und aufgewachsen ist, als Grundlage für den Namen KITAQGB zu wählen.

Der Name **KITAQGB** verbindet zwei Bedeutungen:

- **Kernel-Informed Toolchain for AI-Quality Game Boy Development** beschreibt das Ziel einer Toolchain, die die Zielhardware kennt und sowohl menschliche Programmierer als auch Arbeitsabläufe mit generativer KI unterstützt.
- **KITAQ + GB** verbindet einen örtlichen Kurznamen von **Kitakyushu**, einer Stadt in der **Präfektur Fukuoka in Japan**, mit **Game Boy**. **KITAQ** steht für den Spitznamen **北九 (キタキュー, Kitakyū)**.

Für die englische Aussprache dient **„kee-tah-KYOO“**, IPA **/ˌkiːtɑːˈkjuː/**, als Annäherung an das japanische **キタキュー**. Das abschließende **Q** wird wie der englische Buchstabenname **Q** ausgesprochen. **KITAQGB** lautet auf Englisch **„kee-tah-KYOO jee bee“**; **G** und **B** werden einzeln gesprochen.

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
