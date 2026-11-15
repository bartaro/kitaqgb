# Native Hilfsprogramme

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | [Português (Brasil)](README.pt.md) | **Deutsch**

Alle Hilfsprogramme sind in Rust implementiert und benötigen zur Ausführung weder .NET noch Python/Pillow. Bauen Sie alle Programme mit `cargo build --locked --release`. Unter Windows erhalten die Befehle die Endung `.exe`.


- [GBHUA](gbhua/README.de.md)
- [GBSEQ](gbseq/README.de.md)
- [Handbuch](https://bartaro.github.io/kitaq-docs/de/kitaqgb.html)

```text
kitaqgb-patch-vblank --rom game.gb --map game.map
kitaqgb-zx0 input.bin output.zx0
kitaqgb-zx0 output.zx0 restored.bin --decompress
kitaqgb-zx0 input.bin asset.h --header=level_data
kitaqgb-zx0 input.bin output.kqa --format=auto
```

ZX0 akzeptiert 1 bis 65535 Bytes und bewahrt die Ausgabe des C#-Encoders. Unterstützt werden `raw`, Anzahl/Wert-`rle` und der automatische neun Byte lange `KQA1`-Container. Bei Gleichstand gilt raw, RLE, ZX0. `--decompress` dekodiert einen vorwärts gelesenen ZX0-v2-Strom mit Ausgabelimit. Rückwärtsströme und v1 sind nicht unterstützt. Das Format stammt von Einar Saukas; diese KITAQ-Implementierung steht unter MIT.

Der VBlank-Patcher prüft das Symbol der festen Bank und die PUSH-Signatur, verändert die ROM direkt und berechnet Prüfsummen neu. `--no-header-fix` lässt sie unverändert. Die Suche validiert nicht die gesamte Routine. Erstellen Sie eine Sicherung.
