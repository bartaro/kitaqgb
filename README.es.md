# KITAQGB

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | **Español** | [Português (Brasil)](README.pt-BR.md) | [Deutsch](README.de.md)

[Manual del compilador](https://bartaro.github.io/kitaq-docs/es/kitaqgb.html) · [Manual de las bibliotecas](https://bartaro.github.io/kitaq-docs/es/gb-library.html)

## Qué ofrece KITAQGB

KITAQGB reúne un compilador de C y bibliotecas de apoyo para el desarrollo casero en hardware de la familia Game Boy. Parte de NORCAL y amplía esa base con un flujo de herramientas orientado a la creación de juegos con medios actuales y asistencia de IA.

El proyecto se centra en:

- Compilar código C en ROM de Game Boy.
- Desarrollar software casero para Game Boy y Game Boy Color.
- Proporcionar diagnósticos fáciles de procesar con IA e informes de compilación reproducibles.
- Generar ROM, cabeceras y código de bajo nivel para destinos de la familia LR35902.
- Ofrecer bibliotecas de paletas, tiles, desplazamiento, cámara, audio, comunicación y funciones RPG/ADV/SLG.
- Trabajar con KOKURA CLI para realizar pruebas, obtener trazas y depurar en el emulador.

KITAQGB **no incluye ROM comerciales, BIOS de Nintendo, archivos del SDK de Nintendo, recursos propietarios ni documentación oficial de desarrollo de Nintendo**.

## Plataformas de destino

KITAQGB genera ROM caseras compatibles con Game Boy, compatibles con Game Boy Color o exclusivas de Game Boy Color cuando el proyecto utiliza deliberadamente sus funciones específicas. Las extensiones habituales son:

```text
*.gb
*.gbc
```

Pruebe las ROM generadas en un emulador y, cuando sea posible, en hardware real o con un cartucho flash adecuado. La temporización, las interrupciones, el acceso a VRAM/OAM, el audio, el cable de comunicación y el cambio de bancos requieren especial atención a los detalles del hardware.

## Instalación

Descarga y descomprime el ZIP del repositorio. El compilador y las herramientas incluidos funcionan sin Rust, .NET, Python ni Pillow.

Los ejecutables de Windows están en la raíz del repositorio; los de Linux y macOS, en los directorios bin/ de la tabla siguiente. Conserve lib/ y los avisos de licencia junto a las herramientas. En Linux/macOS use chmod +x y añada el directorio al PATH, o ejecute mediante la ruta completa.

| OS | kitaqgb |
| --- | --- |
| Windows x64 | `./kitaqgb.exe` |
| Linux x64 (static musl) | `bin/linux-x86_64/kitaqgb` |
| Linux x64 (GNU) | `bin/linux-gnu-x86_64/kitaqgb` |
| macOS ARM64 | `bin/macos-arm64/kitaqgb` |
| macOS Intel | `bin/macos-x86_64/kitaqgb` |
| macOS Universal | `bin/macos-universal/kitaqgb` |

## Primeros pasos

Genere un pequeño proyecto de plantilla:

```powershell
.\kitaqgb.exe template hello.c --overwrite
```

Compílelo:

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=dev --fast-build --cache
```

Para utilizar el perfil de publicación:

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=release --cache
```

Ejecute la ROM resultante en su emulador de Game Boy/Game Boy Color habitual, o en KOKURA CLI si desea utilizar las funciones de observación y depuración de la herramienta complementaria.

## Compilar el código

Con Rust 1.85 o posterior puede compilar el compilador y todas las herramientas para Windows, Linux, macOS ARM y macOS Intel. Los ejecutables nativos no necesitan .NET; las herramientas de recursos tampoco requieren Python ni Pillow.

```sh
cargo build --locked --release
```

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
```

```sh
sh scripts/build.sh
```

[Compilador nativo Rust y herramientas auxiliares](tools/README.es.md)

## Utilizar las bibliotecas incluidas

`lib/` contiene código de apoyo reutilizable en C. Compile los archivos de biblioteca junto con el código del juego y añada `-I lib` para que el compilador encuentre las cabeceras.

Este ejemplo combina audio, paletas, desplazamiento, cámara y física. Los comandos de varias líneas que aparecen a continuación usan el carácter de continuación de **cmd.exe**:

```cmd
.\kitaqgb.exe main.c ^
  lib\audio_hwregs_gb.c lib\audio.c ^
  lib\cgb_palette.c lib\scroll.c lib\camera.c ^
  lib\physics2d.c lib\physics2d_circle.c lib\physics3d.c ^
  -I lib -o game.gb --profile=dev --fast-build --cache
```

Ejemplo para proyectos con comunicación serie:

```cmd
.\kitaqgb.exe lib\link_hwregs_gb.c lib\link.c lib\link_packet.c main.c ^
  -I lib -o link_game.gb --profile=dev --fast-build --cache
```

Ejemplo con funciones RPG/ADV/SLG:

```cmd
.\kitaqgb.exe main.c lib\text.c lib\menu.c lib\flags.c lib\script.c lib\map.c lib\save.c ^
  -I lib -o rpg.gb --profile=dev --fast-build --cache
```

Consulte [lib/README.md](lib/README.es.md) para conocer la clasificación de las bibliotecas y sus notas de uso.

### Recursos y compresión

<!-- current-features:start -->
Para comprimir tiles y mapas, consulte la [referencia de ZX0 y su ejemplo visual](https://bartaro.github.io/kitaq-docs/es/gb-library.html#module-zx0), junto con la [herramienta de compresión para el ordenador](tools/zx0/README.es.md).
<!-- current-features:end -->

### Renderizado 3D

Tiempos Wire3D, perfiles de 88 filas y relojes independientes

WIRE3D_DMG_HEIGHT admite 88, 96 o 120; WIRE3DCGB_HEIGHT admite 88 o 96. Los valores predeterminados siguen siendo DMG 120 y CGB 96. El perfil de 88 filas mide 128×88 y su centro Y=44. Use el mismo ajuste en biblioteca y programa; compile wire3d_dmg_88.c / wire3d_cgb_88.c en lugar de la entrada normal. DMG 88 conserva la estructura de modelos de 96 filas y el límite de 16 aristas. El modo CGB 160×144 no cambia.

[Resultados de verificación y ejemplos](https://bartaro.github.io/kitaq-docs/es/gb-library.html#wire3d-feedback-20261009)

## Opciones habituales de la línea de comandos

```text
-o <file>                  Ruta de la ROM de salida
-I <dir>                   Directorio de búsqueda de cabeceras
--profile=dev              Perfil de desarrollo
--profile=release          Perfil de publicación
--fast-build / --fast      Compilación rápida durante el desarrollo
--cache                    Activar la caché de compilación
--no-cache                 Desactivar la caché de compilación
--disasm                   Emitir el desensamblado
--no-disasm                Omitir el desensamblado
--diag-json <file>         Escribir los diagnósticos en JSON
--machine-readable         Preferir una salida procesable por herramientas
--deps-out <file>          Emitir información de dependencias
--debug-output <dir>       Directorio de salidas auxiliares y de depuración
--strict                   Tratar determinadas advertencias como errores
--permissive               Relajar determinados diagnósticos
--stack-bank=fixed|wramx1  Seleccionar el modelo de banco de la pila
--stack-top=<addr>         Elegir la dirección superior de la pila
--stack-reserve=<bytes>    Reservar espacio para la pila
```

Para conocer la lista exacta de opciones admitidas por su versión:

```powershell
.\kitaqgb.exe --help
```

## Flujo de trabajo con KOKURA CLI

KOKURA CLI es el emulador y depurador complementario de KITAQGB. Un ciclo de trabajo habitual es:

1. Escribir o generar el código C del juego.
2. Compilarlo con KITAQGB.
3. Ejecutar la ROM generada en KOKURA CLI.
4. Recoger diagnósticos, trazas, símbolos, observaciones de temporización e informes del emulador.
5. Utilizar esos resultados en la siguiente iteración de código y depuración.

En el desarrollo asistido por IA, este proceso permite convertir un error del compilador, un informe del emulador o una traza de ejecución en una tarea de reparación concreta.

## Criterios de diseño

KITAQGB no pretende ser un compilador de C moderno de propósito general. Está diseñado para desarrollar software casero en una plataforma de juego de 8 bits, con poca memoria, bancos y requisitos de temporización estrictos.

Se busca generar código predecible, ofrecer diagnósticos claros y ejemplos pequeños y reproducibles, y producir informes que puedan leer tanto personas como herramientas de IA. Se mantiene el control de bajo nivel cuando hace falta, junto con bibliotecas de alto nivel donde resultan útiles. Así, el desarrollo para Game Boy se hace más accesible sin ocultar por completo la máquina.

## Colaborar en el desarrollo

Antes de enviar cambios, tenga en cuenta lo siguiente:

- No añada ROM, BIOS, recursos extraídos de juegos comerciales ni material de SDK oficiales protegido por derechos de autor.
- Mantenga los resultados de compilación —como `bin/`, `obj/`, `target/`, `dist/`, `*.exe`, `*.dll` y `*.pdb`— fuera de los commits de código fuente, salvo que haya una razón de publicación concreta.
- Para errores del compilador o de generación de código, procure aportar pruebas pequeñas y reproducibles.
- Al añadir bibliotecas, documente el comando de compilación y las declaraciones de registros de hardware necesarias.
- Redacte diagnósticos lo bastante claros para que las personas y las herramientas de programación con IA puedan actuar sobre ellos.

## Prompt para desarrollar un juego

[Manual](https://bartaro.github.io/kitaq-docs/es/loop-engineering.html#gb)

## Origen del nombre

KITAQGB nació como una bifurcación de NORCAL, el compilador de C para NES vinculado a Zachtronics. Conserva el aviso de derechos de autor de Keith Holman, autor original de NORCAL.

NORCAL toma su nombre del norte de California (Northern California). Inspirándose en esa referencia geográfica, el autor eligió el nombre KITAQGB a partir de Kitakyushu, la ciudad donde nació y creció. KITAQ + GB combina Game Boy con **北九 (キタキュー, Kitakyū)**, el apodo de Kitakyushu, en la prefectura de Fukuoka, Japón. KITAQ se pronuncia como el japonés «キタキュー»; la guía de pronunciación en inglés es **kee-tah-KYOO**, con transcripción fonética **/ˌkiːtɑːˈkjuː/**. La Q final suena como el nombre de la letra Q en inglés. KITAQGB se lee **kee-tah-KYOO jee bee**, pronunciando G y B por separado en inglés.

El nombre KITAQGB tiene dos significados. **Kernel-Informed Toolchain for AI-Quality Game Boy Development** expresa el objetivo de una cadena de herramientas que entiende la máquina de destino y apoya tanto a quienes programan como a la IA generativa.

El otro significado es **Kids' Imagination Transformed into Actual Quests in Game Boy Forests**: una herramienta que convierte la imaginación infantil en aventuras reales en los bosques de Game Boy. Expresa el deseo de transformar pequeñas ideas, garabatos y prototipos creados con ayuda de la IA en aventuras que se puedan jugar de verdad.

## Estado del proyecto: versión preliminar pública

KITAQGB y KOKURA se ofrecen actualmente como herramientas de desarrollo en versión preliminar pública.

Pueden utilizarse para experimentar, crear ejemplos, desarrollar juegos con ayuda de IA, investigar compiladores, depurar mediante emuladores y validar flujos de trabajo. Siguen en desarrollo activo: las API, las opciones de la CLI, los formatos de salida, los diagnósticos y el comportamiento pueden variar entre versiones.

Las versiones preliminares pueden contener errores, funciones incompletas o cambios incompatibles. Antes de utilizarlas en un proyecto de producción o en una publicación, compruebe el código generado, el comportamiento del emulador, los diagnósticos de temporización y los informes.

**Kernel-Informed Toolchain for AI-Quality Game Boy Development**

KITAQGB es una cadena de herramientas de C de código abierto para desarrollar software casero para Game Boy y Game Boy Color. Está pensada para un desarrollo asistido por IA y apoyado en diagnósticos: escribir programas pequeños en C, compilarlos como imágenes ROM `.gb` o `.gbc` y aprovechar la información del emulador para mejorar el juego.

KITAQGB no está afiliado a Nintendo ni cuenta con su respaldo, patrocinio o aprobación. Game Boy y Game Boy Color son marcas de Nintendo.

## Manuales y licencias

- [Manual del compilador](https://bartaro.github.io/kitaq-docs/es/kitaqgb.html) · [Manual de las bibliotecas](https://bartaro.github.io/kitaq-docs/es/gb-library.html)
- [Archivos del manual para consultarlo sin conexión](https://github.com/bartaro/kitaq-docs)
- [Licencia](LICENSE) / [Traducción japonesa de referencia](LICENSE.ja)

La licencia del proyecto no sustituye las condiciones de terceros sobre tipografías, dependencias, logotipos o marcas. Conserve los avisos adjuntos al redistribuir el software.

## Marcas e independencia del proyecto

KITAQGB es un proyecto independiente de código abierto para desarrollo casero. No está afiliado a Nintendo ni cuenta con su respaldo, patrocinio o aprobación. Game Boy y Game Boy Color son marcas de Nintendo.

No añada al repositorio logotipos de Nintendo, ilustraciones de embalajes oficiales, tipografías oficiales, BIOS, datos de ROM comerciales ni recursos propietarios de juegos sin disponer de los derechos necesarios.

## Licencia

KITAQGB se distribuye bajo la licencia MIT. El aviso original de NORCAL, del que deriva, es:

```text
Copyright 2019 Keith Holman
```

Las modificaciones y aportaciones de KITAQGB llevan este aviso:

```text
Copyright (c) 2026 DAISUKE OBA
```

Las copias o partes sustanciales del software deben conservar el aviso de derechos de autor de NORCAL y el texto de la licencia MIT. Consulte [LICENSE](LICENSE) y [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

[Manual](https://bartaro.github.io/kitaq-docs/es/kitaqgb.html)
