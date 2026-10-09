# Native Hilfsprogramme

[en](README.en.md) · [ja](README.ja.md) · [ko](README.ko.md) · [zh-CN](README.zh-CN.md) · [zh-TW](README.zh-TW.md) · [fr](README.fr.md) · [es](README.es.md) · [de](README.de.md)

Alle Hilfsprogramme sind in Rust implementiert und benötigen zur Ausführung weder .NET noch Python/Pillow. Bauen Sie alle Programme mit `cargo build --locked --release`. Unter Windows erhalten die Befehle die Endung `.exe`.

```text
kitaqgb-zx0 input.bin output.zx0
kitaqgb-zx0 output.zx0 restored.bin --decompress
kitaqgb-zx0 input.bin asset.h --header=level_data
kitaqgb-zx0 input.bin output.kqa --format=auto
kitaqgb-patch-vblank --rom game.gb --map game.map
```

ZX0 akzeptiert 1 bis 65535 Bytes und bewahrt die Ausgabe des C#-Encoders. Unterstützt werden `raw`, Anzahl/Wert-`rle` und der automatische neun Byte lange `KQA1`-Container. Bei Gleichstand gilt raw, RLE, ZX0. `--decompress` dekodiert einen vorwärts gelesenen ZX0-v2-Strom mit Ausgabelimit. Rückwärtsströme und v1 sind nicht unterstützt. Das Format stammt von Einar Saukas; diese KITAQ-Implementierung steht unter MIT.

Der VBlank-Patcher prüft das Symbol der festen Bank und die PUSH-Signatur, verändert die ROM direkt und berechnet Prüfsummen neu. `--no-header-fix` lässt sie unverändert. Die Suche validiert nicht die gesamte Routine. Erstellen Sie eine Sicherung.

<!-- wire3d-feedback:start -->

Wire3D-Zeitmessung, Profile mit 88 Zeilen und unabhängige Takte

WIRE3D_DMG_HEIGHT erlaubt 88, 96 oder 120; WIRE3DCGB_HEIGHT erlaubt 88 oder 96. Die Vorgaben bleiben DMG 120 und CGB 96. Das 88-Zeilen-Profil hat 128×88 Pixel mit Mittelpunkt Y=44. Bibliothek und Aufrufer benötigen denselben Wert; kompilieren Sie wire3d_dmg_88.c / wire3d_cgb_88.c statt des normalen Einstiegspunkts. DMG 88 verwendet die Modellstruktur des 96-Zeilen-Profils und dessen Grenze von 16 Kanten. CGB 160×144 bleibt unverändert.

[Prüfergebnisse und Beispiele](https://bartaro.github.io/kitaq-docs/de/gb-library.html#wire3d-feedback-20261009)

Die Wire3D-Mess- und Regressionstests verwenden Python 3. Emulatorprüfungen benötigen die KOKURA-Python-Brücke und die C-API-DLL. Pillow ist für PNG-Ausgaben optional.

<!-- wire3d-feedback:end -->
