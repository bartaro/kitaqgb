# HARAPEKO SHIROHEBI

<!-- readme-language-links:start -->
[English](README.md#english) | [日本語](README.md#%E6%97%A5%E6%9C%AC%E8%AA%9E) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | **Español** | [Deutsch](README.de.md)
<!-- readme-language-links:end -->

Un juego para conseguir la máxima puntuación en Game Boy / Game Boy Color, creado por **DAISUKE OBA**. Dirige a la serpiente blanca, come granadas para crecer y evita las minas y tu propio cuerpo.

- **Presentación y descarga oficial de la ROM:** <https://bartaro.itch.io/harapeko-shirohebi>
- **Guía de programación:** [HTML en español](https://bartaro.github.io/kitaq-docs/apps/harapeko_shirohebi/guide-es.html)
- **Licencia:** [MIT](LICENSE), copyright © 2026 DAISUKE OBA. Abarca el código del juego, los gráficos originales incluidos, los datos de caracteres, la música, los efectos sonoros y la documentación de este directorio. Conserva el aviso de licencia al redistribuirlos. KITAQGB y sus dependencias mantienen los avisos de la [licencia del repositorio](../../LICENSE), incluido el [aviso de la fuente ASCII original](../../licenses/fonts/ASCII-font-MIT.txt).

La guía HTML contiene un diagrama de flujo, el algoritmo de seguimiento de la serpiente, ejemplos de bibliotecas y una guía de los archivos fuente. El enlace anterior abre directamente la página de KITAQ Docs en el navegador. El HTML y la hoja de estilos se mantienen en el [repositorio kitaq-docs](https://github.com/bartaro/kitaq-docs/tree/main/apps/harapeko_shirohebi).

## Compilar en Windows

Con Rust 1.85 o posterior puede compilar el compilador y todas las herramientas para Windows, Linux, macOS ARM y macOS Intel. Los ejecutables nativos no necesitan .NET; las herramientas de recursos tampoco requieren Python ni Pillow. Los scripts PowerShell usan el ejecutable Windows de la raíz. En Linux/macOS pase las mismas entradas C y opciones al compilador nativo, o use PowerShell 7. Los gráficos CHR y el código C son entradas distintas; font.chr conserva la fuente de los ejemplos.

Desde la raíz del repositorio:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\apps\harapeko_shirohebi\build.ps1
```

O desde el directorio del juego:

```powershell
.\build.ps1
# If the game is stored separately, select a complete KITAQGB installation:
.\build.ps1 -KitaqgbRoot C:\tools\kitaqgb
# Retain compiler intermediates for debugging:
.\build.ps1 -KeepBuildFiles
```

La salida predeterminada es `out/shirohebi.gb`, junto con `out/shirohebi.map` y `out/build_manifest.json`. Utiliza `-OutputDirectory C:\build\shirohebi` para elegir otro destino. Si la compilación termina correctamente, se elimina el directorio temporal, salvo que se indique `-KeepBuildFiles`; si falla, se conserva para el diagnóstico. Git ignora los archivos generados.

El script une los fragmentos del juego, compila con las bibliotecas del repositorio, comprueba que las rutinas de interrupción estén en ROM fija, instala el vector VBlank y actualiza las sumas de comprobación del cartucho. Produce una **ROM MBC5 de 64 KiB con 8 KiB de RAM respaldada por batería**, jugable en modos DMG y CGB. No compiles los fragmentos por separado ni omitas el paso del vector y las sumas de comprobación.

Este directorio de la aplicación **no contiene ejecutables ni ROM precompilados**. El compilador se encuentra en la raíz del repositorio; descarga la ROM publicada del juego desde itch.io.

## Controles

| Pantalla | Operación |
|---|---|
| Título | START: empezar. UP/DOWN/SELECT: elegir MUSIC o SOUND. LEFT/RIGHT/A: activar o desactivar la opción seleccionada. |
| Juego | LEFT/RIGHT: girar respecto a la orientación de la serpiente. Mantener UP: acelerar. START: pausa. |
| Pausa | START: reanudar. SELECT: abrir la confirmación de vuelta al título. |
| Confirmación | LEFT/RIGHT/SELECT: elegir. A: confirmar. B/START: cancelar. |
| Nombre | UP/DOWN: elegir A–Z o un punto. LEFT/RIGHT/SELECT: mover el cursor. A: avanzar o terminar en el tercer carácter. START: terminar. |
| Reintento | LEFT/RIGHT/SELECT: elegir YES/NO. A/START: confirmar. |

En el título, SELECT+START abre la confirmación para borrar las puntuaciones. Tras cambiar de pantalla, hay que soltar todos los botones antes de que se acepte una nueva orden. Consulta la guía HTML para conocer las transiciones de estado y los detalles de implementación.
