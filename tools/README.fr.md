# Outils auxiliaires natifs

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | **Français** | [Español](README.es.md) | [Português (Brasil)](README.pt.md) | [Deutsch](README.de.md)

Tous les outils sont implémentés en Rust et fonctionnent sans .NET, Python ni Pillow. Compilez tous les exécutables avec `cargo build --locked --release`. Sous Windows, ajoutez `.exe` aux noms des commandes.


- [GBHUA](gbhua/README.fr.md)
- [GBSEQ](gbseq/README.fr.md)
- [Manuel](https://bartaro.github.io/kitaq-docs/fr/kitaqgb.html)

```text
kitaqgb-patch-vblank --rom game.gb --map game.map
kitaqgb-zx0 input.bin output.zx0
kitaqgb-zx0 output.zx0 restored.bin --decompress
kitaqgb-zx0 input.bin asset.h --header=level_data
kitaqgb-zx0 input.bin output.kqa --format=auto
```

ZX0 accepte 1 à 65535 octets et conserve la sortie du codeur C#. Les formats `raw`, `rle` nombre/valeur et le conteneur automatique `KQA1` de neuf octets sont pris en charge. À égalité : raw, RLE, puis ZX0. `--decompress` décode un flux ZX0 v2 direct avec une taille bornée. Les flux inversés et v1 ne sont pas pris en charge. Einar Saukas a conçu le format ; cette implémentation KITAQ est sous licence MIT.

Le correcteur VBlank vérifie le symbole de banque fixe et la signature PUSH, puis modifie la ROM sur place et recalcule les sommes de contrôle. `--no-header-fix` les conserve. La recherche ne valide pas toute la routine. Faites une sauvegarde.
