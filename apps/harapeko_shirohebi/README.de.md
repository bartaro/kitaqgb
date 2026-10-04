# HARAPEKO SHIROHEBI

<!-- readme-language-links:start -->
[English](README.md#english) | [日本語](README.md#%E6%97%A5%E6%9C%AC%E8%AA%9E) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | **Deutsch**
<!-- readme-language-links:end -->

Ein Spiel für Game Boy / Game Boy Color von **DAISUKE OBA**, bei dem es um die höchste Punktzahl geht. Steuern Sie die weiße Schlange, fressen Sie Granatäpfel zum Wachsen und meiden Sie Minen sowie den eigenen Körper.

- **Spielvorstellung und offizieller ROM-Download:** <https://bartaro.itch.io/harapeko-shirohebi>
- **Programmierleitfaden:** [HTML auf Deutsch](https://bartaro.github.io/kitaq-docs/apps/harapeko_shirohebi/guide-de.html)
- **Lizenz:** [MIT](LICENSE), Copyright © 2026 DAISUKE OBA. Sie gilt für die Spielquellen, mitgelieferten eigenen Grafiken, Schriftdaten, Musik, Soundeffekte und Dokumentation in diesem Verzeichnis. Bei Weitergabe ist der Lizenzhinweis beizubehalten. Für KITAQGB und seine Abhängigkeiten gelten weiterhin die Hinweise in der [Repositorylizenz](../../LICENSE), einschließlich des [Hinweises zur ursprünglichen ASCII-Schrift](../../licenses/fonts/ASCII-font-MIT.txt).

Der HTML-Leitfaden enthält ein Ablaufdiagramm, den Nachführungsalgorithmus der Schlange, Bibliotheksbeispiele und einen Wegweiser durch die Quelldateien. Über den obigen Link lässt sich die KITAQ-Docs-Seite direkt im Browser lesen. HTML und Stylesheet werden im [Repository kitaq-docs](https://github.com/bartaro/kitaq-docs/tree/main/apps/harapeko_shirohebi) gepflegt.

## Unter Windows bauen

Mit Rust 1.85 oder neuer lassen sich Compiler und sämtliche Hilfswerkzeuge für Windows, Linux, macOS ARM und macOS Intel bauen. Die nativen Programme benötigen kein .NET; die Werkzeuge für Grafikdaten benötigen auch weder Python noch Pillow. Die PowerShell-Skripte verwenden das Windows-Programm im Stammverzeichnis. Unter Linux/macOS übergeben Sie dieselben C-Eingaben und Optionen an den nativen Compiler oder verwenden PowerShell 7. CHR-Grafik und C-Quellen sind getrennte Eingaben; font.chr bewahrt die Schrift der Beispiele.

Im Stammverzeichnis des Repositorys:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\apps\harapeko_shirohebi\build.ps1
```

Oder im Verzeichnis dieses Spiels:

```powershell
.\build.ps1
# If the game is stored separately, select a complete KITAQGB installation:
.\build.ps1 -KitaqgbRoot C:\tools\kitaqgb
# Retain compiler intermediates for debugging:
.\build.ps1 -KeepBuildFiles
```

Die Standardausgabe ist `out/shirohebi.gb`, begleitet von `out/shirohebi.map` und `out/build_manifest.json`. Mit `-OutputDirectory C:\build\shirohebi` wählen Sie ein anderes Ziel. Erfolgreiche Builds löschen ihr temporäres Kompilierungsverzeichnis, sofern `-KeepBuildFiles` nicht gesetzt ist; bei Fehlern bleibt es zur Diagnose erhalten. Ausgabedateien werden von Git ignoriert.

Das Skript fügt die aufgeteilten Spielquellen zusammen, kompiliert mit den Repositorybibliotheken, prüft die Platzierung der Interruptroutinen in der festen ROM, installiert den VBlank-Vektor und aktualisiert die Modulprüfsummen. Es erzeugt eine **64-KiB-MBC5-ROM mit 8 KiB batteriegepuffertem RAM**, die im DMG- und CGB-Modus spielbar ist. Kompilieren Sie die Fragmente nicht einzeln und lassen Sie den Schritt für Vektor und Prüfsummen nicht aus.

Dieses Anwendungsverzeichnis enthält **keine vorgefertigte ausführbare Datei und keine ROM**. Der Compiler liegt im Stammverzeichnis des Repositorys; die veröffentlichte Spiel-ROM erhalten Sie auf itch.io.

## Steuerung

| Bildschirm | Bedienung |
|---|---|
| Titel | START: beginnen. UP/DOWN/SELECT: MUSIC oder SOUND wählen. LEFT/RIGHT/A: gewählte Option ein- oder ausschalten. |
| Spiel | LEFT/RIGHT: relativ zur Blickrichtung der Schlange drehen. UP halten: beschleunigen. START: pausieren. |
| Pause | START: fortsetzen. SELECT: Bestätigung zur Titelrückkehr öffnen. |
| Bestätigung | LEFT/RIGHT/SELECT: wählen. A: bestätigen. B/START: abbrechen. |
| Namenseingabe | UP/DOWN: A–Z oder einen Punkt wählen. LEFT/RIGHT/SELECT: Cursor bewegen. A: weiter oder beim dritten Zeichen abschließen. START: abschließen. |
| Neue Runde | LEFT/RIGHT/SELECT: YES/NO wählen. A/START: bestätigen. |

Im Titel öffnet SELECT+START die Bestätigung zum Löschen der Bestenliste. Nach einem Bildschirmwechsel müssen alle Tasten losgelassen werden, bevor ein neuer Befehl angenommen wird. Zustandswechsel und Implementierungsdetails erklärt der HTML-Leitfaden.
