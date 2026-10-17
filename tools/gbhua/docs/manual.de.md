# GBHUA

## Download und Start

Laden Sie unter [GUI-Downloads](../GUI.md) das passende Archiv herunter und entpacken Sie es vollständig. Unter Windows öffnen Sie `GBHUA.exe`; der Ordner `assets` muss daneben bleiben. Auf dem Mac kopieren Sie `GBHUA.app` nach Programme. Wählen Sie Apple Silicon oder Intel passend zum Rechner. Zum Starten sind Rust und Python nicht erforderlich. Die App hat weder Developer-ID-Signatur noch Notarisierung. Verwenden Sie das offizielle Repository. Falls macOS sie blockiert, prüfen Sie die Herkunft und erlauben Sie nur diese App unter Systemeinstellungen > Datenschutz & Sicherheit. Deaktivieren Sie nicht den Systemschutz.

Die GUI verwendet englische Beschriftungen; dieses Dokument ist das übersetzte Handbuch. Auch auf dem Mac bezeichnet `Ctrl` die Control-Taste. Die folgenden Build-Befehle beginnen im Stammverzeichnis des Repositorys. Die CLI benötigt weiterhin weder Bildschirm noch GPU.

[English](manual.en.md) | [日本語](manual.ja.md) | [한국어](manual.ko.md) | [简体中文](manual.zh-CN.md) | [繁體中文](manual.zh-TW.md) | [Español](manual.es.md) | [Português](manual.pt.md) | [Français](manual.fr.md) | [Deutsch](manual.de.md)

GBHUA wandelt PNG-Dateien in Game-Boy-Kacheln, Paletten und Karten um. Autor: DAISUKE OBA. Eigener Programmcode: MIT.

## Bauen und starten

```powershell
cd tools/gbhua
cargo run --release --locked --features gui --bin gbhua_gui
```

## GUI

Ein unverändertes neues Dokument löst keine Warnung vor ungespeicherten Änderungen aus; tatsächliche Änderungen bleiben geschützt. Die Windows-GUI öffnet standardmäßig keine Konsole. Zur Diagnose: `cargo run -p gbhua --bin gbhua_gui --features gui-console`.

Überschriften und obere Leinwandkanten sind ausgerichtet. Die Symbole für Öffnen, Speichern und Rückgängig/Wiederholen zeigen beim Darüberfahren ihren Namen. Die Größenschaltfläche über Graphic wählt die Abmessungen; Einpassen, Minus und Plus über Map steuern den Zoom. DMG/CGB steht oben rechts. Unter 1180px Breite wechseln Sie links zwischen Tiles und Palettes. Paletten und Diagnosen lassen sich per Mausrad oder Scrollbalken verschieben. Escape schließt das Menü.

1. Links eine Kachel und unter Graphic > 8x8 eine der sieben Größen wählen. Große Grafiken verwenden aufeinanderfolgende Kacheln. Farbe wählen und zum Zeichnen ziehen; Rechtsklick nimmt eine Farbe auf.

2. TOOLS > FILL füllt einen zusammenhängenden Bereich, TILES > FILL TILE die ganze Grafik. FLIP X/Y und ROTATE transformieren; EDIT > COPY/PASTE GRAPHIC dupliziert. UNDO/REDO speichert bis zu 64 Schritte.

3. TOOLS > MAP platziert die Grafik; Rechtsklick übernimmt Kachel/Palette. RECT setzt 3x3; FILL füllt; ATTR setzt Palette und schaltet X-Spiegelung; COLL/EVT schaltet Markierungen. VIEW bietet Fit/1x/2x/4x, Raster und Markierungen. Mausrad und Scrollleisten verschieben die Ansicht.

4. In BYTECODE mit Ctrl+A und Ctrl+V 16 Bytes einfügen. IMPORT ändert die erste 8x8-Kachel, EXPORT aktualisiert den Text. Pfeile/Home/End bewegen den Cursor, Shift markiert, Ctrl+C/X kopiert/schneidet aus, Backspace/Delete löscht.

5. FILE > SAVE und EXPORT fragen nach dem Ziel. Bei ungespeicherten Änderungen (*) fragt Öffnen/Import/Schließen nach: Yes speichert, No verwirft, Cancel bearbeitet weiter. PNG-Import ersetzt das Projekt und ist rückgängig zu machen.

6. SPRITES > NEW META erstellt einen Metasprite. ADD/DUP/CLEAR FRAME und PREV/NEXT FRAME verwalten Frames. In METASPRITE fügt Klick Teile hinzu, Rechtsklick entfernt sie. DMG / CGB vergleicht DMG/CGB ohne gespeicherte Paletten zu ändern.

```text
0x78,0x78,0xCC,0xCC,0xCC,0xCC,0xC0,0xC0,
0xDC,0xDC,0xCC,0xCC,0x7C,0x7C,0x00,0x00
```

Vollständige Projekte als .gbh speichern. Bei alten JSON-Projekten genügt die Umbenennung der Endung in .gbh; ein Leser für die alte Endung ist nicht enthalten. GBTD/GBR/GBTB und GBMB/GBM sind Austauschformate und erhalten nicht alle Felder. GBMB erzeugt zusätzlich eine gleichnamige .gbr-Datei; beide zusammen aufbewahren.

## PNG / Python / CLI

Bildmaße müssen Vielfache von 8 sein (8..2040). --size skaliert mit nächstem Nachbarn. DMG nutzt vier Helligkeitsstufen. CGB nähert RGB555 mit vier Farben je Kachel und höchstens acht Paletten an. Transparenz wird mit Weiß verrechnet. Gleiche Kacheln werden geteilt; mehr als 256 unterschiedliche Kacheln werden abgelehnt. Warnungen und Vorschau prüfen.

```powershell
python -m venv .venv
.\.venv\Scripts\python -m pip install maturin
.\.venv\Scripts\python -m maturin develop --features python
```

```python
import gbhua
project = gbhua.Project.from_image("artwork.png", mode="cgb")
project.save("scene.gbh")
project.save_preview("preview.png", "cgb", 2)
print(project.validate_json())
```

[CLI: Deutsch](../README.de.md)

## PNG-Import und Fehlerbehebung

Verwenden Sie `FILE > IMPORT PNG` oder ziehen Sie eine PNG-Datei ins Fenster. Wählen Sie vorher DMG/CGB. Der Import ersetzt das Projekt; speichern Sie zuerst. Die automatische CGB-Palettenwahl berücksichtigt Farbfehler, um seltene Glanzlichter und dunkle Konturen zu erhalten. Die Game-Boy-Grenzen gelten weiterhin. Verlorene Glanzlichter einer alten Konvertierung erfordern einen erneuten Import der Original-PNG; eine bereits reduzierte `.gbh` enthält die verlorenen Pixel nicht mehr. `--size` ist eine CLI-Option.

Wählen Sie die Kachel, klicken Sie in BYTECODE, geben Sie genau 16 Bytes ein und drücken Sie IMPORT. EXPORT aktualisiert nur den Text. Bei abweichender Kartenfarbe oder Ausrichtung prüfen Sie Palette und Spiegelattribute. `.gbh` erhält alle Bearbeitungsdaten. Fehler stehen in der Statusleiste; das Abbrechen eines Dialogs speichert nichts.

Unter macOS/Linux: `python3 -m venv .venv`, `source .venv/bin/activate`, `python -m pip install maturin`, danach `python -m maturin develop --features python`. Zum Bauen der Bindings sind Python 3.10+ und Rust erforderlich.
