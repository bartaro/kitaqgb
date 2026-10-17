# GBHUA

## Descarga e inicio

Descargue el archivo de su sistema en [Descargas GUI](../GUI.md) y extraiga todo su contenido. En Windows abra `GBHUA.exe` conservando la carpeta `assets` a su lado. En Mac copie `GBHUA.app` a Aplicaciones; elija Apple Silicon o Intel según su equipo. No necesita Rust ni Python para ejecutar la GUI. La aplicación no tiene firma Developer ID ni notarización. Use el repositorio oficial; si macOS la bloquea, verifique su origen y autorice solo esta aplicación en Ajustes del Sistema > Privacidad y seguridad. No desactive la protección del sistema.

La GUI tiene etiquetas en inglés; este documento es el manual traducido. En Mac, `Ctrl` también significa Control. Ejecute los comandos de compilación desde la raíz del repositorio. La CLI sigue funcionando sin pantalla ni GPU.

[English](manual.en.md) | [日本語](manual.ja.md) | [한국어](manual.ko.md) | [简体中文](manual.zh-CN.md) | [繁體中文](manual.zh-TW.md) | [Español](manual.es.md) | [Português](manual.pt.md) | [Français](manual.fr.md) | [Deutsch](manual.de.md)

GBHUA convierte PNG en tiles, paletas y mapas para Game Boy. Autor: DAISUKE OBA. Código original: MIT.

## Compilar y ejecutar

```powershell
cd tools/gbhua
cargo run --release --locked --features gui --bin gbhua_gui
```

## GUI

Un documento nuevo sin editar no muestra el aviso de cambios sin guardar; las ediciones reales sí. La GUI de Windows no abre una consola por defecto. Para diagnóstico: `cargo run -p gbhua --bin gbhua_gui --features gui-console`.

Los títulos y los bordes superiores de los lienzos están alineados. Pase el ratón sobre los iconos de abrir, guardar y deshacer/rehacer para ver sus nombres. El botón de tamaño sobre Graphic cambia las dimensiones; ajustar, menos y más sobre Map cambian el zoom. DMG/CGB está arriba a la derecha. Con menos de 1180px de ancho, cambie entre Tiles y Palettes en la barra lateral izquierda. Use la rueda o la barra de desplazamiento para ver las paletas y los diagnósticos. Escape cierra el menú.

1. Seleccione un tile a la izquierda y uno de los siete tamaños en Graphic > 8x8. Los gráficos grandes usan tiles consecutivos. Elija un color, arrastre para pintar y pulse el botón derecho para recoger un color.

2. TOOLS > FILL rellena una región conectada; TILES > FILL TILE, todo el gráfico. FLIP X/Y y ROTATE lo transforman; EDIT > COPY/PASTE GRAPHIC lo duplica. UNDO/REDO conserva hasta 64 operaciones.

3. TOOLS > MAP coloca el gráfico. Botón derecho: tile/paleta. RECT coloca 3x3; FILL rellena; ATTR aplica paleta e invierte X; COLL/EVT alterna marcas. VIEW ofrece Fit/1x/2x/4x, cuadrícula y marcas; use rueda y barras para desplazarse.

4. En BYTECODE, Ctrl+A y Ctrl+V pegan 16 bytes. IMPORT cambia el primer tile 8x8; EXPORT actualiza el texto. Flechas/Home/End mueven el cursor, Shift selecciona, Ctrl+C/X copia/corta y Backspace/Delete borra.

5. FILE > SAVE y EXPORT preguntan el destino. Con cambios sin guardar (*), abrir/importar/cerrar pregunta: Yes guarda, No descarta, Cancel sigue editando. Importar PNG reemplaza el proyecto y se puede deshacer.

6. SPRITES > NEW META crea un metasprite. ADD/DUP/CLEAR FRAME y PREV/NEXT FRAME gestionan fotogramas. En METASPRITE, clic coloca piezas y botón derecho las elimina. DMG / CGB compara DMG/CGB sin cambiar paletas almacenadas.

```text
0x78,0x78,0xCC,0xCC,0xCC,0xCC,0xC0,0xC0,
0xDC,0xDC,0xCC,0xCC,0x7C,0x7C,0x00,0x00
```

Guarde el proyecto completo como .gbh. Para proyectos JSON antiguos basta cambiar la extensión a .gbh; no hay lector de la extensión anterior. GBTD/GBR/GBTB y GBMB/GBM son formatos de intercambio y no conservan todos los campos. GBMB también genera un .gbr del mismo nombre: conserve ambos.

## PNG / Python / CLI

Las dimensiones deben ser múltiplos de 8 (8..2040). --size redimensiona por vecino más cercano. DMG usa cuatro tonos; CGB aproxima RGB555, cuatro colores por tile y hasta ocho paletas. La transparencia se compone sobre blanco. Se comparten tiles idénticos y se rechazan más de 256 tiles únicos. Revise avisos y vista previa.

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

[CLI: Español](../README.es.md)

## Importación PNG y solución de problemas

Use `FILE > IMPORT PNG` o arrastre un PNG. Seleccione DMG/CGB antes de importarlo. La importación sustituye el proyecto: guarde primero. La selección automática CGB considera el error de color para conservar reflejos poco frecuentes y contornos oscuros. Siguen vigentes los límites de Game Boy. Para recuperar reflejos perdidos en una importación antigua, importe de nuevo el PNG original; el `.gbh` reducido no contiene los píxeles perdidos. `--size` es una opción de la CLI.

Seleccione la baldosa, haga clic dentro de BYTECODE, escriba exactamente 16 bytes y pulse IMPORT. EXPORT solo actualiza el texto. Si el mapa tiene otros colores u orientación, compruebe paleta y atributos de inversión. Guarde en `.gbh` para conservar todos los datos. Los errores aparecen en la barra de estado; cancelar el diálogo no guarda.

En macOS/Linux ejecute `python3 -m venv .venv`, `source .venv/bin/activate`, `python -m pip install maturin` y `python -m maturin develop --features python`. Compilar los bindings requiere Python 3.10+ y Rust.
