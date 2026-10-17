# GBSEQ — Benutzerhandbuch

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBSEQ von DAISUKE OBA bietet eine plita-GUI und eine unabhängige CLI. Beide verwenden dasselbe Modell, dieselbe Prüfung, KOKURA-Audioausgabe und Exportlogik. LOCAL 4CH, CABLE 8CH und DMG07 16CH stehen für 1, 2 und 4 Game Boy. Jeder Spieler hat CH1/CH2 Puls, CH3 Wellenform und CH4 Rauschen. Ein Stück umfasst 1–256 Takte mit je 16 Schritten (4096 je Kanal), BPM 30–300 und eine Schleifenoption. Die Oberfläche ist englisch; Handbücher gibt es in neun Sprachen.

## Installation

Wählen Sie unter [bin/](bin/) Windows x86_64, Linux x86_64 oder macOS Apple Silicon/Intel. Entpacken Sie das gesamte Archiv; assets und Lizenzdateien bleiben neben den Programmen. GUI: gbseq.exe unter Windows, ./gbseq unter macOS/Linux. Die Beispiele nennen gbseq-cli; verwenden Sie in PowerShell ./gbseq-cli.exe und unter Unix ./gbseq-cli, sofern es nicht im PATH liegt. Für die normale Nutzung benötigen Sie weder Compiler noch eine separate plita-Installation oder Python.

Die GUI benötigt Desktop, passenden GPU-Treiber und ein Audiogerät zum Anhören. Linux zielt auf Ubuntu 24.04 oder neuere kompatible Systeme mit ALSA, X11/Wayland, libxkbcommon und Vulkan/OpenGL-Laufzeitbibliotheken. Die macOS-Dateien sind nicht signiert oder notarisiert: Herkunft und Hash prüfen und gegebenenfalls nur diese App über Öffnen/Datenschutz & Sicherheit freigeben; den globalen Schutz nicht abschalten. Die CLI erzeugt WAV ohne Anzeige oder Soundgerät. Falls unter Unix nötig: chmod +x gbseq gbseq-cli.

## GUI-Bearbeitung

Modus und P1–P4 wählen. ALL16 in DMG07 zeigt und spielt alle sechzehn Kanäle. Doppelklick auf eine leere Spur erstellt eine Note, auf eine Note öffnet den Inspektor. Ziehen des Körpers ändert Zeit/Tonhöhe, der rechten Kante die Dauer. Jede Spur ist einstimmig; Überlappungen werden abgewiesen. Taktzahl 1–256. Mausrad/untere Leiste verschieben die Ansicht; F passt das Stück ein, Home springt zum Anfang, +/- zoomt. Die Ansicht folgt der Wiedergabe. Space startet/stoppt, LOOP ON/OFF schaltet die Schleife. Beim ersten Start wird das gesamte Stück berechnet; lange Stücke benötigen Vorbereitungszeit.

Ctrl/Shift-Klick ändert die Auswahl; Ctrl+A wählt die Spur. Links/rechts verschiebt im Raster, oben/unten in Halbtönen, Shift+oben/unten in Oktaven. Ctrl+D dupliziert, Delete löscht, G wechselt das Raster, Q richtet Anfänge aus. Ctrl+Z/Y macht rückgängig/wiederholt, Ctrl+C/V kopiert/fügt ein. Ctrl+O öffnet, Ctrl+S speichert, Ctrl+Shift+S speichert unter. Vor dem Ersetzen ungespeicherter Änderungen erfolgt eine Abfrage; vor Überschreiben eine Kopie behalten.

## Klang und Wellen

Pro Note sind Pegel, Panorama links/rechts/Mitte, Pulstastverhältnis und Hüllkurve, CH1-Sweep, CH3-Pegel/Preset und CH4-Rauschen einstellbar. AUDITION spielt die Note vor. CH3 besitzt keine Hardware-Hüllkurve. NEW USER WAVE bearbeitet 32 Vier-Bit-Werte (0–15) durch Ziehen. 64 Benutzerwellen und sieben eingebaute Wellen lassen sich 16 aktiven Presets mit 100/50/25% Pegel zuweisen. WAVE PRESET automatisiert den Klang je Note. Ein Hardware-Wellenwechsel schaltet CH3 kurz ab; es ist keine kontinuierliche Überblendung.

## CLI

track/player zählen ab 1, start ab 0. length 1 ist eine Sechzehntel, 16 ein 4/4-Takt. pitch erlaubt 0–47, velocity 0–15. players unterstützt 1, 2 oder 4. Bearbeitungen erfordern --out; das ausdrücklich gewählte Ziel kann überschrieben werden. Ungültige Bearbeitungen enden mit Fehlerstatus und werden nicht gespeichert. transform unterstützt auch --shift, --quantize und --velocity. --all-players mischt alle aktiven Spieler, sonst nur P1. Exporte können vorhandene Ausgaben ersetzen.

```sh
gbseq --open song.gbseq
gbseq-cli new song.gbseq --bars 32 --bpm 150 --players 4 --no-loop
gbseq-cli add-note song.gbseq --track 1 --start 0 --length 2 --pitch 24 --velocity 12 --out edited.gbseq
gbseq-cli notes edited.gbseq --track 1
gbseq-cli transform edited.gbseq --track 1 --transpose 12 --out higher.gbseq
gbseq-cli duplicate-notes higher.gbseq --ids 1 --out doubled.gbseq
gbseq-cli delete-notes doubled.gbseq --ids 1 --out trimmed.gbseq
gbseq-cli validate edited.gbseq
gbseq-cli info edited.gbseq --json
gbseq-cli render-wav edited.gbseq all16.wav --all-players
gbseq-cli export-kitaqgb edited.gbseq export-c --player 1
gbseq-cli export-link16-sav edited.gbseq saves
gbseq-cli bench-render edited.gbseq --all-players --frames 60 --runs 3
gbseq-cli --help
gbseq-cli transform --help
```


## LINK16- und KITAQGB-Export

LINK16 erzeugt 128-KiB-MBC5-Batterieabbilder V4: link16_local.sav; link16_cable.p1.sav/.p2.sav; oder link16_dmg07.p1.sav–.p4.sav. Passende LINK16-ROM/Version und Emulator-Dateinamen verwenden. Vorhandene SAV sichern. P1 enthält auch Spiegel der anderen Spieler. Keine LINK16-ROM wird mitgeliefert; SAV allein ist nicht startfähig. Pro Spieler gibt es vier gemeinsam genutzte Hüllkurvenpresets; weitere Werte können angenähert werden. Nach dem Import anhören. V4 speichert keinen CH1-Sweep. frames_per_step_override unterstützt bildgenaue Effekte in Vorschau/WAV/KITAQGB, wird vom BPM-basierten LINK16-Export jedoch abgewiesen.

KITAQGB exportiert C/Stream eines Spielers; für jeden Spieler wiederholen. Kompatible audio.h/audio.c mit CH1=0, CH2=1, CH3=2, CH4=3 verwenden. Sechzehn Kanäle passen dadurch nicht in einen einzigen Game Boy. Die Vorschau rundet Schrittdauern auf Videoframes; manche BPM unterscheiden sich von ROMs mit gebrochenem Taktgeber. Emulation ist kein Hardwaretest.

## Daten und Python

.gbseq enthält JSON, kein MIDI; Umbenennen der Endung konvertiert nichts. KI-Daten validieren und Länge, aktive Kanäle, eindeutige IDs, Tonumfang und Überlappungen prüfen. Die [Detailspezifikation](docs/README.md) ist japanisch. Für die optionale Python-3.10+-Anbindung python/ in PYTHONPATH aufnehmen und GBSEQ_NATIVE_LIBRARY auf den absoluten Pfad von gbseq.dll/libgbseq.dylib/libgbseq.so setzen. Python-Indizes beginnen bei null; nach direkten Änderungen an data validate aufrufen. player=None mischt alle Spieler.

```python
from gbseq import Project
song = Project()
song.data["measures"] = 4
song.add_note(track=0, start=0, length=2, pitch=24, velocity=12)
song.validate()
song.save("python-song.gbseq")
song.render_wav("python-song.wav")
```


## Bauen, Fehlerbehebung und Rechte

Rust 1.93.1 und die C/C++-Werkzeuge des Betriebssystems verwenden. Linux benötigt libasound2-dev, libudev-dev, libxkbcommon-dev, libwayland-dev, libx11-dev, libxi-dev, libxcursor-dev und Mesa-Entwicklungsbibliotheken. CI prüft Modell, Editor, Audio/SAV und GUI-Start je OS; verification dokumentiert den Quell-Commit. Physische Audioausgabe, Eingaben und Game-Boy-Hardware werden dabei nicht geprüft.

```sh
cd tools/gbseq
cargo test --locked --release --tests
cargo build --locked --release --bin gbseq
cargo build --locked --release --no-default-features --bin gbseq-cli --lib
```


Startfehler: GPU-Treiber und Terminalmeldung prüfen. Fehlender Text: assets/fonts; kein Ton: Ausgabegerät und mute/solo; Exportfehler: validate und angezeigte Note. GBSEQ steht unter MIT, Copyright (c) 2026 DAISUKE OBA. [LICENSE](LICENSE), [Dritthinweise](THIRD_PARTY_NOTICES.md), [Audit](LICENSE_AUDIT.json) und Schriftlizenzen bei Weitergabe erhalten. Drittmaterial und importierte Musik behalten ihre jeweiligen Rechte.
