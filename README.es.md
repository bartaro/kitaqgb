# KITAQGB

[English](README.md#english) | [日本語](README.md#japanese) | **Español**

**[Manual del compilador en español](https://bartaro.github.io/kitaq-docs/es/kitaqgb.html)** · **[Manual de las bibliotecas en español](https://bartaro.github.io/kitaq-docs/es/gb-library.html)**

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

## Organización del repositorio

El código del compilador se encuentra en el subdirectorio homónimo `kitaqgb/`. El ejecutable Release y su configuración de ejecución están en la raíz; las bibliotecas y los ejemplos tienen sus propios directorios.

```text
kitaqgb/                  # Repository root
├─ kitaqgb/               # Compiler build sources
│  ├─ *.cs
│  ├─ app.config
│  └─ kitaqgb.csproj
├─ kitaqgb.exe            # Prebuilt Release compiler
├─ kitaqgb.exe.config     # .NET Framework runtime configuration
├─ lib/                # C support libraries
├─ examples/           # Tutorial programs and original font
├─ scripts/build.ps1   # Rebuild the Release executable
├─ LICENSE
└─ LICENSE.ja
```

El compilador incluido requiere Windows y .NET Framework 4.8. Descargue el ZIP del repositorio para conservar juntos el ejecutable, su configuración, las bibliotecas y los avisos de licencia. Para recompilar también necesita .NET Framework 4.8 Developer Pack y Visual Studio Build Tools. Ejecute desde la raíz:

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
.\examples\build.ps1
```

La compilación Release copia el ejecutable y su configuración a la raíz. La versión Debug permanece en `kitaqgb/bin/Debug` y no sobrescribe el compilador Release distribuido. No se incluyen cachés de compilación ni archivos PDB. Consulte las entradas y sus valores SHA-256 en el [registro de compilación del binario](BINARY_BUILD.json).

## Requisitos de desarrollo

El entorno principal es Windows, con los archivos de referencia de .NET Framework 4.8 y Visual Studio o Visual Studio Build Tools con MSBuild. Se utiliza el formato clásico de proyecto C#, destinado a `.NET Framework v4.8`.

Es posible que otros sistemas funcionen con Mono/MSBuild, según los ensamblados de referencia instalados, pero la vía de compilación principal sigue siendo Windows con MSBuild.

## Compilar KITAQGB

Desde la raíz del repositorio:

```powershell
msbuild kitaqgb\kitaqgb.csproj /p:Configuration=Release
```

Si la compilación termina correctamente, el proyecto copia el ejecutable a la raíz:

```text
kitaqgb.exe
```

A continuación, consulte la ayuda de la línea de comandos:

```powershell
.\kitaqgb.exe --help
```

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

Consulte la clasificación y las notas de las bibliotecas en [lib/README.es.md](lib/README.es.md).

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

<!-- development-prompt:es:start -->
## Prompt para desarrollar un juego

Completa los requisitos y entrega el prompt íntegro a tu asistente de IA. Incluye implementación, pruebas en el emulador, análisis con SARAKURA y verificación de las correcciones.

[Leer el ejemplo práctico en el manual HTML](https://bartaro.github.io/kitaq-docs/es/kitaqgb.html#loop-prompts)

<details>
<summary>Mostrar el prompt completo</summary>

### Desarrollo de juegos con KITAQGB, KOKURA y SARAKURA

Completa los requisitos y entrega este documento íntegro al asistente de IA. Los comandos suponen que los repositorios `kitaqgb`, `kitaqfc`, `kokura`, `kurosaki`, `sarakura` y `kitaq-docs`, junto con el proyecto `game-gb` o `game-fc`, comparten un directorio padre. Ejecútalos desde ese directorio y adapta las rutas al entorno real.

#### Requisitos

- Título del juego: &lt;completar&gt;
- Género y mecánica principal: &lt;completar&gt;
- Controles y condiciones de éxito y fracaso: &lt;completar&gt;
- Pantallas, niveles, enemigos y objetos obligatorios: &lt;completar&gt;
- Estilo visual, música y efectos de sonido: &lt;completar e indicar los recursos proporcionados&gt;
- Guardado, comunicación, periféricos y otros requisitos: &lt;completar o ninguno&gt;
- Directorio del proyecto: &lt;completar&gt;
- Condiciones de redistribución: &lt;por ejemplo, código y recursos originales aptos para publicarse con licencia MIT&gt;

- Máquina de destino: &lt;Game Boy original / compatibilidad con GB y CGB / solo CGB&gt;
- Rendimiento: &lt;por ejemplo, 60 actualizaciones de la lógica por segundo durante el juego normal; definir lo aceptable en escenas exigentes&gt;

#### Trabajo solicitado

Implementa el juego con KITAQGB y sus bibliotecas. Utiliza KOKURA para ejecutar y depurar, y SARAKURA para organizar los diagnósticos y comparar los resultados antes y después de una corrección.

Repite este ciclo hasta cumplir los criterios de aceptación: concretar la especificación → implementar un cambio pequeño → compilar → aplicar entradas y observar → investigar la causa → corregir → repetir las pruebas en las mismas condiciones. Un plan, un listado de código o una compilación correcta no bastan para dar el trabajo por terminado.

##### Comprobar el entorno y los criterios de aceptación

1. Lee las instrucciones del directorio de trabajo, los README, los manuales HTML y las cabeceras e implementaciones de las bibliotecas que vayas a usar. Registra las rutas de los ejecutables y sus versiones o hashes SHA-256. Comprueba los comandos con la salida real de `--help` y las API con el código fuente.
2. Define criterios verificables para entradas, imagen, sonido, progreso y frecuencia de actualización. Por ejemplo: pulsar y soltar START inicia la partida; una colisión resta una vida; la pausa silencia el audio indicado y al continuar se reanuda la reproducción.
3. Pregunta solo por ambigüedades importantes. Resuelve de forma autónoma las decisiones habituales y reversibles. No rebajes los requisitos ni los criterios de aceptación.
4. Primero ejecuta un pequeño ejemplo incluido con el compilador, el emulador y SARAKURA. Esto comprueba la conexión entre herramientas, no la finalización del juego solicitado.

##### Implementar una primera versión jugable

- Utiliza el dialecto C de KITAQGB y `void main()`. No des por disponibles las API de C de escritorio o GBDK. Incluye los archivos `.c` necesarios, no solo sus declaraciones; comprueba inicialización, unidades, signo, rangos, vida útil de los búferes y bancos ROM.
- Planifica las actualizaciones de VRAM/OAM, VBlank, interrupciones, pila, bancos ROM/WRAM y límites de tiles y sprites. La capacidad total y libre de la cola de transferencias es distinta de la capacidad y el espacio libre de la VRAM física.
- Un juego para DMG no debe depender de funciones exclusivas de CGB. Si admite ambos modos, pruébalos por separado.
- Para letras, números y símbolos, utiliza la fuente original proporcionada en `ascii.c` y comprueba la correspondencia entre caracteres y tiles.

- Conecta primero arranque, título, personaje controlable, éxito o fracaso y reinicio. Amplía el contenido después.
- Conserva los originales editables de gráficos, música y efectos, así como los pasos de generación. Comprueba que la compilación consume realmente los datos exportados.
- Escribe los comentarios del código en inglés y los informes de progreso en español. Mantén los informes estándar de SARAKURA en inglés.

##### Vincular cada compilación con su ejecución

Separa las salidas por iteración, por ejemplo en `out/iter-001`. Registra comandos, códigos de salida y hashes de código, recursos, herramientas, ROM y metadatos. Nunca ejecutes una ROM anterior después de una compilación fallida. Los mapas, mapas de código fuente y datos de depuración deben corresponder a la misma compilación que la ROM.

Este es un ejemplo de comprobación básica para DMG. Prepara `main.c` y las implementaciones de biblioteca necesarias; adapta opciones y secuencia de entrada al juego.

```powershell
$iteration = '.\game-gb\out\iter-001'
New-Item -ItemType Directory -Force $iteration | Out-Null

# Include all additional implementation units required by the game.
& '.\kitaqgb\kitaqgb.exe' '.\game-gb\src\main.c' `
  -I '.\kitaqgb\lib' -o "$iteration\game.gb" `
  --profile=dev --rst-disable --stack-bank=fixed --no-disasm `
  "--emit-ai-metadata=$iteration\build.json"
if ($LASTEXITCODE -ne 0) { throw 'Build failed; inspect the build log.' }

# This sequence presses START once, with released intervals on both sides.
& '.\kokura\kokura-cli.exe' "$iteration\game.gb" `
  --hardware dmg --run-frames 300 `
  --input-seq 'NONE:60;START:1;NONE:239' `
  --png "$iteration\frame.png" --record-wav "$iteration\audio.wav" `
  --dump-report "$iteration\run.json" `
  --emit-diagnostics "$iteration\events.jsonl"
if ($LASTEXITCODE -ne 0) { throw 'Emulator run failed; inspect the run log.' }

& '.\sarakura\sarakura.exe' gb analyze `
  --metadata "$iteration\build.json" --events "$iteration\events.jsonl" `
  --frames 300 --out "$iteration\analysis" --fail-on error
if ($LASTEXITCODE -ne 0) { throw 'Inspect the analysis report and fix the cause.' }
```


`--hardware dmg` selecciona la Game Boy original. Al probar CGB o ambos modos, ajusta de forma coherente la cabecera ROM y el hardware del emulador. La secuencia pulsa START una vez entre intervalos con los botones sueltos. Ejecutar 300 fotogramas no equivale a probar el juego completo.

##### Comprobar imagen, sonido, estado y rendimiento

- Guarda escenarios que distingan pulsar, mantener y soltar. Recorre todas las rutas especificadas: arranque, inicio, movimiento, acciones, colisiones, desplazamiento, cambios de nivel, fin de partida, reinicio, pausa y, cuando corresponda, guardado o comunicación.
- Conserva PNG de fotogramas relevantes, entradas, informes de ejecución, JSONL de diagnóstico, WAV y las observaciones necesarias de estado o memoria. Comprueba los fotogramas alcanzados y el motivo de parada. Abre las imágenes: una sola captura no demuestra movimiento ni respuesta a los controles. Compara contadores, posiciones y cambios de estado con lo esperado; revisa bordes de pantalla, límites de tiles y atributos, y escenas con muchos sprites.
- Comprueba música, efectos, reproducción simultánea, cortes, pausa y reanudación. Crear un WAV no demuestra que el sonido sea correcto. Si no puedes escucharlo, distingue las comprobaciones numéricas o de forma de onda de las cualidades audibles aún sin verificar.
- Mide escenas exigentes, trabajo de la CPU de destino, actualizaciones y transferencias; en FC, incluye el trabajo de NMI. La velocidad del emulador en el equipo anfitrión no es la frecuencia del juego ni prueba la velocidad en hardware real. Continuar con `--allow-unimplemented` no demuestra soporte para la función ausente.

##### Analizar, corregir y volver a probar

- Proporciona a SARAKURA los metadatos de la ROM probada y el JSONL de diagnóstico de esa ejecución. Una traza CPU o un informe ordinario no los sustituyen. `--frames` establece condiciones de análisis; SARAKURA no ejecuta la ROM ni modifica automáticamente el código.
- Lee `report.html`, `ai_diagnostics.json`, `repair_prompt.md` y `retest_plan.json`. Contrasta los diagnósticos con reproducción, imágenes, audio y código. Distingue ubicaciones o causas inferidas de hechos comprobados, y bucles de espera normales de bloqueos. Evalúa las advertencias una por una y registra eventos no soportados o límites del análisis. No ocultes advertencias con filtros ni acortes las pruebas para conseguir un resultado favorable.
- Reduce cada fallo a un caso mínimo, corrige su causa y recompila. Si procede del compilador o emulador, aísla el defecto del código del juego y añade comprobaciones de regresión para la corrección de la herramienta.
- Repite las pruebas con las mismas entradas, semilla aleatoria, máquina y norma de vídeo, mapper, fotogramas observados y ajustes de diagnóstico. Cada ROM requiere sus metadatos; no reutilices estados guardados a ciegas tras cambiar código o distribución de RAM.

```powershell
& '.\sarakura\sarakura.exe' baseline-delta `
  --baseline '.\game-gb\out\iter-001\analysis' `
  --current '.\game-gb\out\iter-002\analysis' `
  --out '.\game-gb\out\delta.json' --markdown '.\game-gb\out\delta.md' `
  --fail-on-new error --fail-on-regression error --enforce
```


Usa las diferencias de diagnóstico junto con la aceptación de controles, gráficos y audio. Si el mismo fallo se repite, revisa las pruebas y la hipótesis en lugar de encadenar cambios arbitrarios.

##### Criterios de finalización y entregables

Repite todos los escenarios obligatorios con la ROM final compilada a partir del código y los ajustes entregados. La invencibilidad, entradas automáticas de prueba u otro mapper, por sí solos, no verifican una partida normal en la versión final. Entrega una tabla de requisitos y pruebas, explica las advertencias restantes e identifica lo no comprobado o no soportado. Si no se ha probado en hardware físico, indícalo explícitamente.

Entrega código fuente, identificación de herramientas y bibliotecas, recursos editables, scripts reproducibles de compilación y pruebas, ROM, evidencia final y un README de instalación, controles y limitaciones conocidas. Incluye reproducciones y el programa de pruebas cuando sean necesarios. Publica o envía archivos al exterior solo dentro del alcance autorizado expresamente. Elimina compilaciones intermedias y trazas temporales innecesarias tras verificarlas, conservando fuentes, recursos, entregables y evidencia de regresión necesaria.

Si el entorno o los permisos impiden una comprobación obligatoria, comunica los pasos exactos de reproducción y la acción necesaria. No des el trabajo por terminado.

</details>
<!-- development-prompt:es:end -->

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

## Colaborar en el desarrollo

Antes de enviar cambios, tenga en cuenta lo siguiente:

- No añada ROM, BIOS, recursos extraídos de juegos comerciales ni material de SDK oficiales protegido por derechos de autor.
- Mantenga los resultados de compilación —como `bin/`, `obj/`, `target/`, `dist/`, `*.exe`, `*.dll` y `*.pdb`— fuera de los commits de código fuente, salvo que haya una razón de publicación concreta.
- Para errores del compilador o de generación de código, procure aportar pruebas pequeñas y reproducibles.
- Al añadir bibliotecas, documente el comando de compilación y las declaraciones de registros de hardware necesarias.
- Redacte diagnósticos lo bastante claros para que las personas y las herramientas de programación con IA puedan actuar sobre ellos.

## Estado de esta publicación

Este repositorio se ha preparado para la primera publicación de KITAQGB. Las interfaces, las bibliotecas, los diagnósticos y la integración con herramientas complementarias pueden evolucionar a medida que madure el proyecto.

## Compilación y primer uso

En Windows, utilice .NET Framework 4.8 Developer Pack y MSBuild de Visual Studio Build Tools. Ejecute desde Developer PowerShell:

```powershell
MSBuild.exe .\kitaqgb\kitaqgb.csproj /t:Build /p:Configuration=Release
.\kitaqgb.exe --help
.\examples\build.ps1
```

## Manuales y licencias

- [Compilador: manual en español](https://bartaro.github.io/kitaq-docs/es/kitaqgb.html) / [Bibliotecas: manual en español](https://bartaro.github.io/kitaq-docs/es/gb-library.html)
- [Archivos del manual para consultarlo sin conexión](https://github.com/bartaro/kitaq-docs)
- [Licencia](LICENSE) / [Traducción japonesa de referencia](LICENSE.ja)

La licencia del proyecto no sustituye las condiciones de terceros sobre tipografías, dependencias, logotipos o marcas. Conserve los avisos adjuntos al redistribuir el software.
