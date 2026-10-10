# GBHUA

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBHUA convierte PNG en tiles, paletas y mapas para Game Boy. Autor: DAISUKE OBA. Código original: MIT.

## Compilar y ejecutar

```sh
cd tools/gbhua
cargo build --release --locked
./target/release/gbhua --help
```

Windows: `target\release\gbhua.exe`. Rust >= 1.92.

## Flujo de trabajo

Importe el PNG, consulte el informe JSON compacto, valide, previsualice y exporte. Cada comando correcto emite un objeto JSON. Para reducir tokens, dé a la IA la ruta y el informe, no el proyecto completo.

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

Las dimensiones deben ser múltiplos de 8 (8..2040). --size redimensiona por vecino más cercano. DMG usa cuatro tonos; CGB aproxima RGB555, cuatro colores por tile y hasta ocho paletas. La transparencia se compone sobre blanco. Se comparten tiles idénticos y se rechazan más de 256 tiles únicos. Revise avisos y vista previa.

Guarde el proyecto completo como .gbh. Para proyectos JSON antiguos basta cambiar la extensión a .gbh; no hay lector de la extensión anterior. GBTD/GBR/GBTB y GBMB/GBM son formatos de intercambio y no conservan todos los campos. GBMB también genera un .gbr del mismo nombre: conserve ambos.

Sobrescribir requiere --force. Las rutas de entrada y salida deben ser distintas. Códigos: 0 éxito, 1 proyecto inválido, 2 error de argumentos/E/S/conversión. No incluye API de generación de imágenes ni envío por red. Las dependencias conservan sus licencias; consulte LICENSE, THIRD_PARTY_NOTICES.md y DEPENDENCIES.json.

## GUI / Python

La GUI local y los enlaces Python comparten el núcleo. La GUI no se incluye en el paquete CLI independiente. En el entorno local completo ejecute gbhua_gui; en Python use import gbhua. La GUI ofrece menús agrupados, diálogos de guardado, comparación DMG/CGB, siete tamaños, entrada de 16 bytes, deshacer/rehacer, zoom y desplazamiento del mapa.

