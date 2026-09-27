# ZX0-kompatible Ressourcenkompression

<!-- readme-language-links:start -->
[English](README.md#english) | [日本語](README.md#%E6%97%A5%E6%9C%AC%E8%AA%9E) | [한국어](README.ko.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | **Deutsch**
<!-- readme-language-links:end -->

[API und Beispiele](https://bartaro.github.io/kitaq-docs/de/gb-library.html#module-zx0)

Der PC-Kompressor und der GB-Dekompressor sind eigenständige KITAQ-Implementierungen für vorwärts gelesene ZX0-v2-Datenströme. Die KITAQ-Implementierung steht unter der MIT-Lizenz, Copyright (c) 2026 DAISUKE OBA.

Das ZX0-Format und der ursprüngliche Kompressionsalgorithmus wurden von [Einar Saukas](https://github.com/einar-saukas/ZX0) entworfen. Diese Würdigung des Formats ist von Urheberrecht und Lizenz der KITAQ-Implementierung getrennt. Siehe [LICENSE](../../LICENSE) und [LICENSE.ja](../../LICENSE.ja).

Bauen Sie das PC-Werkzeug im Stammverzeichnis des Repositorys:

```powershell
.\tools\zx0\build.ps1
.\kitaqgb-zx0.exe input.bin output.zx0
.\kitaqgb-zx0.exe input.bin asset.h --header=level_data
.\kitaqgb-zx0.exe output.zx0 restored.bin --decompress
```

Das eigenständige Werkzeug verwendet .NET Framework 4.x und akzeptiert 1 bis 65535 Eingabebyte. Es nutzt eine begrenzte Hashketten-Suche und garantiert keine optimale komprimierte Größe. Die Ausgabe besteht aus gewöhnlichen ZX0-v2-Daten ohne KITAQ-Hülle. Rückwärtsströme, Präfixwörterbücher und ZX0 v1 sind nicht Teil dieser Schnittstelle. Die Rechte an den Ressourcen verbleiben bei ihren Urhebern.

Jede kodierte Ausgabe muss in den Größenparameter der Ziel-API mit maximal 65535 Byte passen. Bei automatischen Containern zählt der neun Byte lange Header dazu. Teilen Sie größere Ressourcen auf und beachten Sie zusätzlich die deutlich kleineren RAM- und Bankfenstergrenzen des Zielsystems. Ein leerer raw-C-Header enthält ein Platzhalterbyte, hat aber die logische `_SIZE` null. Ein leerer reiner ZX0-Strom wird nicht unterstützt.

Um Rohdaten, Anzahl/Wert-RLE und ZX0 zu vergleichen und die kleinste Nutzlast zu wählen:

```powershell
.\kitaqgb-zx0.exe input.bin output.kqa --format=auto
```

Der Automatikmodus fügt einen neun Byte langen KQA1-Header hinzu und meldet den gewählten Codec. Er vergleicht Nutzlastgrößen; bei Gleichstand gilt die Reihenfolge raw, RLE, ZX0. Dies ist ein KITAQ-Ressourcencontainer, kein reiner ZX0-Strom. Der Header besteht aus `KQA1`, einem Codecbyte (0 raw, 1 RLE, 2 ZX0), der Originalgröße als u16 in Little-Endian-Reihenfolge und der Nutzlastgröße im selben Format. Direkt danach folgen die Nutzdaten. Verwenden Sie dafür `asset_decompress`. `--format=raw` und `--format=rle` geben nur die gewählte Nutzlast aus; bei RLE beendet eine Anzahl von null den Strom.

Binden Sie `zx0.h` ein und kompilieren Sie `lib/zx0.c` für das Ziel. `zx0_decompress` erhält Zieladresse, Zielkapazität, komprimierte Quelle und deren Größe. Prüfen Sie sowohl die zurückgegebene Bytezahl als auch `zx0_error`. Ein Fehler kann eine Teilausgabe hinterlassen; zeigen oder verwenden Sie diese nach einem Fehlschlag nicht. Quell- und Zielpuffer dürfen sich weder überlappen noch Grenzen der aktuell eingeblendeten CPU-Bankfenster überschreiten. Die Routinen verwenden gemeinsamen Arbeitsspeicher und dürfen während ihrer Ausführung nicht aus Interrupts erneut aufgerufen werden.

Puffer dürfen auch nicht über das Ende des CPU-Adressraums hinaus zum Anfang umlaufen.

`zx0_decompress_vram` schreibt nur bei ausgeschaltetem LCD in die gewählte GB-VRAM-Bank. Anzeige-, Bank- und Interrupteinstellungen bleiben erhalten. Übertragen Sie Daten während der Szeneninitialisierung und setzen Sie nicht voraus, dass eine ganze Ressource in einen VBlank passt.
