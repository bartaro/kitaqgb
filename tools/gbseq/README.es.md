# GBSEQ — manual de usuario

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

GBSEQ, de DAISUKE OBA, ofrece una GUI con plita y una CLI independiente. Comparten modelo, validación, síntesis KOKURA y exportación. LOCAL 4CH, CABLE 8CH y DMG07 16CH corresponden a 1, 2 y 4 Game Boy. Cada jugador dispone de CH1/CH2 de pulso, CH3 de onda y CH4 de ruido. Admite 1–256 compases de 16 pasos (4096 por canal), BPM 30–300 y reproducción con o sin bucle. La interfaz está en inglés; hay nueve manuales traducidos.

## Instalación

Descargue en [bin/](bin/) Windows x86_64, Linux x86_64 o macOS Apple Silicon/Intel. Extraiga todo el archivo, conservando assets y las licencias junto a los programas. Abra gbseq.exe en Windows o ./gbseq en macOS/Linux. La CLI es gbseq-cli; en PowerShell use ./gbseq-cli.exe y en Unix ./gbseq-cli si no está en PATH. Los ejemplos omiten ese prefijo. No necesita compilador, otra instalación de plita ni Python para el uso normal.

La GUI requiere escritorio, controlador gráfico compatible y dispositivo de audio para escuchar. Linux se dirige a Ubuntu 24.04 o sistemas compatibles posteriores, con ALSA, X11/Wayland, libxkbcommon y bibliotecas Vulkan/OpenGL. macOS no está firmado ni notarizado: compruebe origen y hash, y si es necesario autorice solo esta aplicación mediante Abrir/Privacidad y seguridad; no desactive la protección global. La CLI genera WAV sin pantalla ni dispositivo de sonido. Si falta el permiso Unix: chmod +x gbseq gbseq-cli.

## Edición gráfica

Seleccione modo y P1–P4. ALL16 en DMG07 muestra y reproduce los dieciséis canales. Doble clic en un carril vacío crea una nota; sobre una nota abre el inspector. Arrastre el cuerpo para mover tiempo/altura y el borde derecho para duración. Cada carril es monofónico: se rechazan solapamientos. Ajuste 1–256 compases; navegue con rueda/barra inferior, F para encajar canción, Home para inicio y +/- para zoom. La vista sigue la reproducción. Space inicia/detiene y LOOP ON/OFF controla el bucle. La primera reproducción renderiza toda la canción y puede tardar.

Ctrl/Shift-clic cambia selección; Ctrl+A selecciona el carril. Izquierda/derecha mueve por rejilla; arriba/abajo transpone semitonos, Shift+arriba/abajo octavas. Ctrl+D duplica, Delete borra, G cambia rejilla, Q alinea comienzos. Ctrl+Z/Y deshace/rehace, Ctrl+C/V copia/pega. Ctrl+O abre, Ctrl+S guarda, Ctrl+Shift+S guarda como. Se confirma antes de sustituir cambios no guardados; conserve copias antes de sobrescribir.

## Timbre y ondas

El inspector ajusta nivel, panorama izquierda/derecha/centro, ciclo de trabajo y envolvente de pulso, barrido CH1, nivel/preset CH3 y ruido CH4 por nota. AUDITION escucha la nota. CH3 no tiene envolvente de hardware. NEW USER WAVE permite dibujar 32 muestras de 4 bits (0–15). Hay 64 ondas de usuario y siete integradas, asignables a 16 presets activos con nivel 100/50/25%. WAVE PRESET automatiza cambios por nota. Cambiar la onda apaga brevemente CH3; no es un fundido continuo.

## CLI

track/player comienzan en 1; start en 0. length 1 es semicorchea, 16 un compás 4/4. pitch admite 0–47 y velocity 0–15. players acepta 1, 2 o 4. Editar exige --out, que puede sobrescribir el destino explícito. Una edición inválida termina con código distinto de cero y no se guarda. transform también acepta --shift, --quantize y --velocity. --all-players mezcla todos; por defecto se usa P1. Exportar puede reemplazar salidas existentes.

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


## Exportación LINK16 y KITAQGB

LINK16 produce imágenes MBC5 de batería V4 de 128 KiB: link16_local.sav; link16_cable.p1.sav/.p2.sav; o link16_dmg07.p1.sav–.p4.sav. Use ROM/versiones LINK16 compatibles y el nombre de guardado del emulador. Respalde los SAV existentes. P1 contiene también copias de las partes remotas. Esta distribución no incluye ROM LINK16; un SAV no arranca por sí solo. Solo hay cuatro presets de envolvente compartidos por jugador: valores adicionales pueden aproximarse. Escuche después de importar. V4 no conserva el barrido CH1. frames_per_step_override permite efectos por fotograma en vista previa/WAV/KITAQGB, pero LINK16 basado en BPM los rechaza.

KITAQGB exporta C/flujo de un jugador: repita por jugador. Use audio.h/audio.c compatibles con CH1=0, CH2=1, CH3=2 y CH4=3. No convierte dieciséis canales en una sola Game Boy. La vista previa cuantiza pasos a fotogramas de vídeo; algunos BPM difieren de ROM con reloj fraccional. Emulación no equivale a prueba física.

## Datos y Python

.gbseq es JSON, no MIDI; renombrar la extensión no convierte formatos. Valide datos de IA y revise duración, canales activos, IDs únicos, altura y solapamientos. Las [especificaciones detalladas](docs/README.md) están en japonés. Para Python 3.10+ opcional añada python/ a PYTHONPATH y establezca GBSEQ_NATIVE_LIBRARY con la ruta absoluta de gbseq.dll/libgbseq.dylib/libgbseq.so. En Python los índices empiezan en cero; llame validate tras editar data directamente. player=None mezcla todos los jugadores.

```python
from gbseq import Project
song = Project()
song.data["measures"] = 4
song.add_note(track=0, start=0, length=2, pitch=24, velocity=12)
song.validate()
song.save("python-song.gbseq")
song.render_wav("python-song.wav")
```


## Compilar, resolver problemas y licencias

Use Rust 1.93.1 y herramientas C/C++ de su OS. Linux necesita libasound2-dev, libudev-dev, libxkbcommon-dev, libwayland-dev, libx11-dev, libxi-dev, libxcursor-dev y desarrollo Mesa. CI comprueba modelos, edición, audio/SAV y arranque GUI por OS; verification registra el commit. No prueba salida física de audio, entrada física ni hardware Game Boy.

```sh
cd tools/gbseq
cargo test --locked --release --tests
cargo build --locked --release --bin gbseq
cargo build --locked --release --no-default-features --bin gbseq-cli --lib
```


Ante fallo de inicio revise el controlador GPU y los mensajes de terminal. Para texto ausente restaure assets/fonts; para silencio revise dispositivo y mute/solo; para exportación fallida use validate y revise la nota indicada. GBSEQ es MIT, Copyright (c) 2026 DAISUKE OBA. Conserve [LICENSE](LICENSE), [avisos de terceros](THIRD_PARTY_NOTICES.md), [auditoría](LICENSE_AUDIT.json) y licencias de fuentes al redistribuir. Los materiales externos y música importada conservan sus propios derechos.
