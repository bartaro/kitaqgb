# GBHUA

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBHUA wandelt PNG-Dateien in Game-Boy-Kacheln, Paletten und Karten um. Autor: DAISUKE OBA. Eigener Programmcode: MIT.

## Bauen und starten

```sh
cd tools/gbhua
cargo build --release --locked
./target/release/gbhua --help
```

Windows: `target\release\gbhua.exe`. Rust >= 1.92.

## macOS-Programme

CLI für Apple Silicon und Intel: [Download und Start](MACOS.md).
Das passende Archiv entpacken, Terminal in diesem Ordner öffnen und `./gbhua --help` ausführen.
In den Beispielen `./gbhua` statt `./target/release/gbhua` verwenden. Rust und Python werden nicht benötigt.
Minimales Build-Ziel: macOS 11.0; auf macOS 15 getestet. Keine Apple-Developer-ID-Signatur oder Notarisierung.

## Arbeitsablauf

PNG importieren, kompakten JSON-Bericht prüfen, validieren, Vorschau erstellen und exportieren. Erfolgreiche Befehle geben ein JSON-Objekt aus. Für weniger Tokens der KI nur Dateipfad und Bericht statt des gesamten Projekts übergeben.

```sh
./target/release/gbhua import-image artwork.png --out scene.gbh --mode cgb --size 160x144
./target/release/gbhua inspect scene.gbh
./target/release/gbhua validate scene.gbh
./target/release/gbhua preview scene.gbh --out preview.png --mode cgb --scale 2
./target/release/gbhua export scene.gbh --out scene.c --prefix scene
./target/release/gbhua export scene.gbh --out tiles.gbtb
./target/release/gbhua export scene.gbh --out world.gbmb
./target/release/gbhua export scene.gbh --out tiles.2bpp
```

Bildmaße müssen Vielfache von 8 sein (8..2040). --size skaliert mit nächstem Nachbarn. DMG nutzt vier Helligkeitsstufen. CGB nähert RGB555 mit vier Farben je Kachel und höchstens acht Paletten an. Transparenz wird mit Weiß verrechnet. Gleiche Kacheln werden geteilt; mehr als 256 unterschiedliche Kacheln werden abgelehnt. Warnungen und Vorschau prüfen.

Vollständige Projekte als .gbh speichern. Bei alten JSON-Projekten genügt die Umbenennung der Endung in .gbh; ein Leser für die alte Endung ist nicht enthalten. GBTD/GBR/GBTB und GBMB/GBM sind Austauschformate und erhalten nicht alle Felder. GBMB erzeugt zusätzlich eine gleichnamige .gbr-Datei; beide zusammen aufbewahren.

Überschreiben erfordert --force. Ein- und Ausgabepfade müssen verschieden sein. Exitcodes: 0 Erfolg, 1 ungültiges Projekt, 2 Argument-/E/A-/Konvertierungsfehler. Keine Bilderzeugungs-API und kein Netzwerk-Upload. Abhängigkeiten behalten ihre Lizenzen; siehe LICENSE, THIRD_PARTY_NOTICES.md und DEPENDENCIES.json.

## GUI / Python

Lokale GUI und Python-Bindings verwenden denselben Kern. Das eigenständige CLI-Paket enthält keine GUI. Im vollständigen lokalen Arbeitsbereich gbhua_gui starten; in Python import gbhua verwenden. Die GUI bietet gruppierte Menüs, Speicherdialoge, DMG/CGB-Vergleich, sieben Größen, 16-Byte-Eingabe, Rückgängig/Wiederholen sowie Kartenzoom und Bildlauf.

