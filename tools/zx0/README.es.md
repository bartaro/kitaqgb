# Herramientas auxiliares nativas

[en](../README.en.md) · [ja](../README.ja.md) · [ko](../README.ko.md) · [zh-CN](../README.zh-CN.md) · [zh-TW](../README.zh-TW.md) · [fr](../README.fr.md) · [es](../README.es.md) · [de](../README.de.md)

Todas las herramientas están implementadas en Rust y se ejecutan sin .NET, Python ni Pillow. Compile todos los ejecutables con `cargo build --locked --release`. En Windows, añada `.exe` a los comandos.

```text
kitaqgb-zx0 input.bin output.zx0
kitaqgb-zx0 output.zx0 restored.bin --decompress
kitaqgb-zx0 input.bin asset.h --header=level_data
kitaqgb-zx0 input.bin output.kqa --format=auto
kitaqgb-patch-vblank --rom game.gb --map game.map
```

ZX0 acepta de 1 a 65535 bytes y conserva la salida del codificador C#. Admite `raw`, `rle` cantidad/valor y el contenedor automático `KQA1` de nueve bytes. En caso de empate: raw, RLE y ZX0. `--decompress` decodifica un flujo ZX0 v2 hacia delante con límite de salida. No admite flujos inversos ni v1. Einar Saukas diseñó el formato; esta implementación KITAQ tiene licencia MIT.

El corrector VBlank comprueba el símbolo del banco fijo y la firma PUSH, modifica la ROM y recalcula las sumas. `--no-header-fix` conserva las sumas originales. La búsqueda no valida toda la rutina. Guarde una copia.

```sh
sh tools/zx0/build.sh
```
