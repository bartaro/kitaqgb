# GBSEQ — Benutzerhandbuch

**2026-11-14 — source update / ソース更新:** Ctrl+K oder das Klaviersymbol öffnet die Tastatur mit kontinuierlicher Audioausgabe. Neu/Öffnen bietet bei ungespeicherten Änderungen Speichern/Verwerfen/Abbrechen im Fenster. In Wiedergabe-ROMs schaltet B die Schleife in SOLO/MASTER um; TIME zeigt Position/Dauer.

Die Wiedergabe erzeugt kurze Audioblöcke nach Bedarf. Monitor, Stumm und Solo behalten die Position bei; Noten- und Tempoänderungen gelten beim nächsten Start.

[Audio implementation and source checks](../../SOURCE_UPDATE_20261114.md). [Native packages updated 2026-11-15](bin/README.md) include this source update. 配布バイナリも更新済みです。

[MIDI-Dateien und Geräte, Bildschirmklavier, Aufnahme und 512 Klangfarben](docs/MIDI_PERFORMANCE.md) · `Ctrl+K` · `import-midi` / `export-midi`

Exportierte ROMs enthalten die 48 Byte der Nintendo-Logodaten zur Kompatibilität mit der Startprüfung. Die MIT-Lizenz von GBSEQ gewährt keine eigenständigen Rechte am Logo oder an den Marken. GBSEQ ist weder mit Nintendo verbunden noch von Nintendo genehmigt. [Hinweis auf Englisch/Japanisch](docs/NINTENDO_NOTICE.md).

GB-Studio-Export: Im Exportmenü UGE (editierbar, mit eigenen Wellen) oder MOD (feste GBT-Klänge) wählen. Pro Konsole entsteht eine separate Datei in `assets/music`. [CLI: `export-uge` / `export-mod`](docs/GB_STUDIO_EXPORT.md).

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBSEQ von DAISUKE OBA bietet eine plita-GUI und eine unabhängige CLI. Beide verwenden dasselbe Modell, dieselbe Prüfung, KOKURA-Audioausgabe und Exportlogik. LOCAL 4CH, CABLE 8CH und DMG07 16CH stehen für 1, 2 und 4 Game Boy. Jeder Spieler hat CH1/CH2 Puls, CH3 Wellenform und CH4 Rauschen. Ein Stück umfasst 1–256 Takte mit je 16 Schritten (4096 je Kanal), BPM 30–300 und eine Schleifenoption. Die Sprache lässt sich unter Ansicht → Sprache wählen. Die Einstellung wird unabhängig vom Stück gespeichert. `--language en|ja|ko|zh-CN|zh-TW|es|pt-BR|fr|de`

## Einspielen, aufnehmen und WAV ausgeben

Zielspur wählen und mit Ctrl+K die Klaviatur öffnen. A/S/D/F/G/H/J spielen die weißen Tasten, W/E/T/Y/U die schwarzen; Z/X wechselt die Oktave. Mit IN und OUT das MIDI-Gerät wählen. Die Schritteingabe rückt um 1, 2, 4 oder 8 Sechzehntel vor. REC nimmt einen Durchgang ab Stückanfang auf. Der Schutz überspringt überlappende Noten; Überschreiben ersetzt nur den belegten Abschnitt. MIDI über Datei importieren und über Export ausgeben. Für GB-Klangfarben das .gbseq-Projekt behalten.

Pro Kanal stehen 128 Werksklänge im Auswahlmenü. Eigene Klänge lassen sich getrennt als .gbpreset speichern und laden. Akkorde können für die Mischung und für jede Spur unabhängig ein- und ausgeblendet werden. Die globale Transposition lässt die Rauschspur unverändert.

Beim WAV-Export gelten die aktuellen Abhör-, Stumm- und Soloschaltungen. Alle abhören mischt alle aktiven Konsolen; P1–P4 isoliert eine. Sobald eine Spur auf Solo steht, sind nur nicht stummgeschaltete Solospuren hörbar. Die Wahl der Bearbeitungsspur ändert die Mischung nicht. Bei vollständig stummgeschalteten Spuren bleibt die WAV-Dauer erhalten.

`gbseq-cli render-wav song.gbseq mix.wav` · `gbseq-cli render-wav song.gbseq p2.wav --player 2`

[VBlank-Treiber und Musik als C-Code exportieren](docs/KITAQGB_VBLANK.md): `gbseq-cli export-vblank song.gbseq output` · `--strict` · `--driver-dir`.
Die Datensätze verwenden die Kanalreihenfolge aus audio.h. Der VBlank-Treiber unterstützt auf CH3 nur Dreieck und Sägezahn. Mit --strict werden Annäherungen abgelehnt. Die Quellen jeder Konsole separat übersetzen.

Mit **E** oder einem Doppelklick auf den Spurkopf öffnet sich der Piano-Roll-Editor der Spur; E führt zurück zur Übersicht. Ziehen Sie den rechten Notenrand bis zur nächsten Note oder zum Stückende. Im CH3-Welleneditor können Sie die 32 Abtastwerte zeichnen; Strg+Z nimmt einen Strich zurück.

## Wiedergabe-ROMs und Abhören

**Export → Playback ROMs (no SRAM)** erzeugt pro Stimmeinheit eine 512-KiB-Datei (P1.gb bis P4.gb) mit Songtitel und vier Kanälen. Es wird kein Spielstand benötigt. START spielt den jeweiligen Part allein ab. Für den Linkbetrieb auf allen angehaltenen Geräten SELECT drücken, auf einem Gerät A (MASTER), auf den anderen B (PEER) wählen. START am Master steuert die Wiedergabe. Beim Vier-Spieler-Adapter gehört der Master unabhängig vom Part an Port 1. In GBSEQ lassen sich alle Konsolen oder einzelne Parts unabhängig von der bearbeiteten Spur abhören; der WAV-Export übernimmt diese Auswahl.

`gbseq-cli export-player-roms song.gbseq output_folder`

## Installation

Wählen Sie unter [bin/](bin/) Windows x86_64, Linux x86_64 oder macOS Apple Silicon/Intel. Entpacken Sie das gesamte Archiv; assets und Lizenzdateien bleiben neben den Programmen. GUI: gbseq.exe unter Windows, ./gbseq unter macOS/Linux. Die Beispiele nennen gbseq-cli; verwenden Sie in PowerShell ./gbseq-cli.exe und unter Unix ./gbseq-cli, sofern es nicht im PATH liegt. Für die normale Nutzung benötigen Sie weder Compiler noch eine separate plita-Installation oder Python.

Die GUI benötigt Desktop, passenden GPU-Treiber und ein Audiogerät zum Anhören. Linux zielt auf Ubuntu 24.04 oder neuere kompatible Systeme mit ALSA, X11/Wayland, libxkbcommon + libxkbcommon-x11 und Vulkan/OpenGL-Laufzeitbibliotheken. Die macOS-Dateien sind nicht signiert oder notarisiert: Herkunft und Hash prüfen und gegebenenfalls nur diese App über Öffnen/Datenschutz & Sicherheit freigeben; den globalen Schutz nicht abschalten. Die CLI erzeugt WAV ohne Anzeige oder Soundgerät. Falls unter Unix nötig: chmod +x gbseq gbseq-cli.

## GUI-Bearbeitung

Modus und P1–P4 wählen. ALL16 in DMG07 zeigt alle sechzehn Kanäle. Doppelklick auf eine leere Spur erstellt eine Note, auf eine Note öffnet den Inspektor. Ziehen des Körpers ändert Zeit/Tonhöhe, der rechten Kante die Dauer. Jede Spur ist einstimmig; Überlappungen werden abgewiesen. Taktzahl 1–256. Mausrad/untere Leiste verschieben die Ansicht; F passt das Stück ein, Home springt zum Anfang, +/- zoomt. Die Ansicht folgt der Wiedergabe. Space startet/stoppt, LOOP ON/OFF schaltet die Schleife. Die Wiedergabe erzeugt kurze Audioblöcke nach Bedarf. Monitor, Stumm und Solo behalten die Position bei; Noten- und Tempoänderungen gelten beim nächsten Start.

Ctrl/Shift-Klick ändert die Auswahl; Ctrl+A wählt die Spur. Links/rechts verschiebt im Raster, oben/unten in Halbtönen, Shift+oben/unten in Oktaven. Ctrl+D dupliziert, Delete löscht, G wechselt das Raster, Q richtet Anfänge aus. Ctrl+Z/Y macht rückgängig/wiederholt, Ctrl+C/V kopiert/fügt ein. Ctrl+O öffnet, Ctrl+S speichert, Ctrl+Shift+S speichert unter. Vor dem Ersetzen ungespeicherter Änderungen erfolgt eine Abfrage; vor Überschreiben eine Kopie behalten.

## Klang und Wellen

Pro Note sind Pegel, Panorama links/rechts/Mitte, Pulstastverhältnis und Hüllkurve, CH1-Sweep, CH3-Pegel/Preset und CH4-Rauschen einstellbar. AUDITION spielt die Note vor. CH3 besitzt keine Hardware-Hüllkurve. NEW USER WAVE bearbeitet 32 Vier-Bit-Werte (0–15) durch Ziehen. 64 Benutzerwellen und sieben eingebaute Wellen lassen sich 16 aktiven Presets mit 100/50/25% Pegel zuweisen. WAVE PRESET automatisiert den Klang je Note. Ein Hardware-Wellenwechsel schaltet CH3 kurz ab; es ist keine kontinuierliche Überblendung.

## CLI

track/player zählen ab 1, start ab 0. length 1 ist eine Sechzehntel, 16 ein 4/4-Takt. pitch erlaubt 0–47, velocity 0–15. players unterstützt 1, 2 oder 4. Bearbeitungen erfordern --out; das ausdrücklich gewählte Ziel kann überschrieben werden. Ungültige Bearbeitungen enden mit Fehlerstatus und werden nicht gespeichert. transform unterstützt auch --shift, --quantize und --velocity. WAV mischt standardmäßig alle aktiven Spieler; --player N isoliert einen. Exporte können vorhandene Ausgaben ersetzen.

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

LINK16 erzeugt 128-KiB-MBC5-Batterieabbilder V4: link16_local.sav; link16_cable.p1.sav/.p2.sav; oder link16_dmg07.p1.sav–.p4.sav. Passende LINK16-ROM/Version und Emulator-Dateinamen verwenden. Vorhandene SAV sichern. P1 enthält auch Spiegel der anderen Spieler. Eine SAV-Datei allein ist nicht startfähig. Pro Spieler gibt es vier gemeinsam genutzte Hüllkurvenpresets; weitere Werte können angenähert werden. Nach dem Import anhören. V4 speichert keinen CH1-Sweep. frames_per_step_override unterstützt bildgenaue Effekte in Vorschau/WAV/KITAQGB, wird vom BPM-basierten LINK16-Export jedoch abgewiesen.

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

Rust 1.93.1 und die C/C++-Werkzeuge des Betriebssystems verwenden. Linux benötigt libasound2-dev, libudev-dev, libxkbcommon-dev, libxkbcommon-x11-dev, libwayland-dev, libx11-dev, libxi-dev, libxcursor-dev und Mesa-Entwicklungsbibliotheken. CI prüft Modell, Editor, Audio/SAV und GUI-Start je OS; verification dokumentiert den Quell-Commit. Physische Audioausgabe, Eingaben und Game-Boy-Hardware werden dabei nicht geprüft.

```sh
cd tools/gbseq
cargo test --locked --release --tests
cargo build --locked --release --bin gbseq
cargo build --locked --release --no-default-features --bin gbseq-cli --lib
```


Startfehler: GPU-Treiber und Terminalmeldung prüfen. Fehlender Text: assets/fonts; kein Ton: Ausgabegerät und mute/solo; Exportfehler: validate und angezeigte Note. GBSEQ steht unter MIT, Copyright (c) 2026 DAISUKE OBA. [LICENSE](LICENSE), [Dritthinweise](THIRD_PARTY_NOTICES.md), [Audit](LICENSE_AUDIT.json) und Schriftlizenzen bei Weitergabe erhalten. Drittmaterial und importierte Musik behalten ihre jeweiligen Rechte.
