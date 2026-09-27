# Compresión de recursos compatible con ZX0

<!-- readme-language-links:start -->
[English](README.md#english) | [日本語](README.md#%E6%97%A5%E6%9C%AC%E8%AA%9E) | [한국어](README.ko.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | **Español** | [Deutsch](README.de.md)
<!-- readme-language-links:end -->

[API y ejemplos](https://bartaro.github.io/kitaq-docs/es/gb-library.html#module-zx0)

El compresor para PC y el descompresor GB son implementaciones independientes de KITAQ para flujos ZX0 v2 de lectura hacia delante. La implementación de KITAQ se distribuye con licencia MIT, copyright (c) 2026 DAISUKE OBA.

[Einar Saukas](https://github.com/einar-saukas/ZX0) diseñó el formato ZX0 y el algoritmo de compresión original. Este reconocimiento del formato es independiente de los derechos de autor y la licencia de la implementación de KITAQ. Consulta [LICENSE](../../LICENSE) y [LICENSE.ja](../../LICENSE.ja).

Compila la herramienta de PC desde la raíz del repositorio:

```powershell
.\tools\zx0\build.ps1
.\kitaqgb-zx0.exe input.bin output.zx0
.\kitaqgb-zx0.exe input.bin asset.h --header=level_data
.\kitaqgb-zx0.exe output.zx0 restored.bin --decompress
```

La herramienta independiente usa .NET Framework 4.x y acepta entradas de 1 a 65535 bytes. Utiliza una búsqueda acotada mediante cadenas hash, por lo que no garantiza el tamaño comprimido óptimo. Produce datos ZX0 v2 normales, sin envoltura KITAQ. Esta interfaz no admite flujos inversos, diccionarios de prefijos ni ZX0 v1. Los derechos de los recursos originales siguen perteneciendo a sus autores.

Toda salida codificada debe caber en el parámetro de tamaño de 65535 bytes de la API de destino. Los contenedores automáticos incluyen sus nueve bytes de cabecera en ese límite. Divide los recursos grandes y respeta también la RAM y las ventanas de banco del destino, mucho menores. Una cabecera C raw vacía contiene un byte de reserva, con `_SIZE` lógico igual a cero. No se admite un flujo ZX0 puro vacío.

Para comparar los datos sin comprimir, RLE por pares cantidad/valor y ZX0, y conservar la carga útil más pequeña:

```powershell
.\kitaqgb-zx0.exe input.bin output.kqa --format=auto
```

El modo automático añade una cabecera KQA1 de nueve bytes e informa del códec elegido. Compara los tamaños de las cargas útiles; en caso de empate, prefiere raw, después RLE y por último ZX0. Es un contenedor de recursos KITAQ, no un flujo ZX0 puro. La cabecera contiene `KQA1`, un byte de códec (0 raw, 1 RLE, 2 ZX0), el tamaño original como u16 little-endian y el tamaño de la carga útil en el mismo formato. El cuerpo va inmediatamente después. Usa `asset_decompress` para este contenedor. `--format=raw` y `--format=rle` solo emiten la carga útil elegida; RLE termina con una cantidad igual a cero.

Incluye `zx0.h` y compila `lib/zx0.c` para el destino. `zx0_decompress` recibe la dirección de destino, su capacidad, el origen comprimido y su tamaño. Comprueba tanto el número de bytes devuelto como `zx0_error`. Un error puede dejar una salida parcial: no la muestres ni la utilices si la llamada falla. Los búferes de origen y destino no deben solaparse ni atravesar los límites de las ventanas de banco de CPU mapeadas. Estas rutinas comparten una zona de trabajo y no deben volver a invocarse desde una interrupción mientras se ejecutan.

Tampoco se permite que los búferes desborden el espacio de direcciones de la CPU y vuelvan al inicio.

`zx0_decompress_vram` escribe en el banco VRAM de GB seleccionado solo con el LCD apagado. Conserva los ajustes de pantalla, banco e interrupciones. Realiza la carga al inicializar una escena; no supongas que un recurso completo cabe en un solo VBlank.
