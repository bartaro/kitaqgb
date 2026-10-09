# Herramientas auxiliares nativas

[en](README.en.md) · [ja](README.ja.md) · [ko](README.ko.md) · [zh-CN](README.zh-CN.md) · [zh-TW](README.zh-TW.md) · [fr](README.fr.md) · [es](README.es.md) · [de](README.de.md)

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

<!-- wire3d-feedback:start -->

Tiempos Wire3D, perfiles de 88 filas y relojes independientes

WIRE3D_DMG_HEIGHT admite 88, 96 o 120; WIRE3DCGB_HEIGHT admite 88 o 96. Los valores predeterminados siguen siendo DMG 120 y CGB 96. El perfil de 88 filas mide 128×88 y su centro Y=44. Use el mismo ajuste en biblioteca y programa; compile wire3d_dmg_88.c / wire3d_cgb_88.c en lugar de la entrada normal. DMG 88 conserva la estructura de modelos de 96 filas y el límite de 16 aristas. El modo CGB 160×144 no cambia.

[Resultados de verificación y ejemplos](https://bartaro.github.io/kitaq-docs/es/gb-library.html#wire3d-feedback-20261009)

Los scripts de medición y regresión Wire3D usan Python 3. Las pruebas de emulador requieren el puente Python KOKURA y su DLL C API. Pillow es opcional para PNG.

<!-- wire3d-feedback:end -->
