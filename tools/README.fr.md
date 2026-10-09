# Outils auxiliaires natifs

[en](README.en.md) · [ja](README.ja.md) · [ko](README.ko.md) · [zh-CN](README.zh-CN.md) · [zh-TW](README.zh-TW.md) · [fr](README.fr.md) · [es](README.es.md) · [de](README.de.md)

Tous les outils sont implémentés en Rust et fonctionnent sans .NET, Python ni Pillow. Compilez tous les exécutables avec `cargo build --locked --release`. Sous Windows, ajoutez `.exe` aux noms des commandes.

```text
kitaqgb-zx0 input.bin output.zx0
kitaqgb-zx0 output.zx0 restored.bin --decompress
kitaqgb-zx0 input.bin asset.h --header=level_data
kitaqgb-zx0 input.bin output.kqa --format=auto
kitaqgb-patch-vblank --rom game.gb --map game.map
```

ZX0 accepte 1 à 65535 octets et conserve la sortie du codeur C#. Les formats `raw`, `rle` nombre/valeur et le conteneur automatique `KQA1` de neuf octets sont pris en charge. À égalité : raw, RLE, puis ZX0. `--decompress` décode un flux ZX0 v2 direct avec une taille bornée. Les flux inversés et v1 ne sont pas pris en charge. Einar Saukas a conçu le format ; cette implémentation KITAQ est sous licence MIT.

Le correcteur VBlank vérifie le symbole de banque fixe et la signature PUSH, puis modifie la ROM sur place et recalcule les sommes de contrôle. `--no-header-fix` les conserve. La recherche ne valide pas toute la routine. Faites une sauvegarde.

<!-- wire3d-feedback:start -->

Mesure Wire3D, profils de 88 lignes et horloges indépendantes

WIRE3D_DMG_HEIGHT accepte 88, 96 ou 120 ; WIRE3DCGB_HEIGHT accepte 88 ou 96. Les valeurs par défaut restent DMG 120 et CGB 96. Le profil de 88 lignes mesure 128×88, avec Y central=44. Utilisez le même réglage dans la bibliothèque et le programme ; compilez wire3d_dmg_88.c / wire3d_cgb_88.c à la place de l’entrée normale. DMG 88 garde la structure des modèles à 96 lignes et la limite de 16 arêtes. Le mode CGB 160×144 reste inchangé.

[Résultats de vérification et exemples](https://bartaro.github.io/kitaq-docs/fr/gb-library.html#wire3d-feedback-20261009)

Les scripts de mesure et de régression Wire3D utilisent Python 3. Les tests émulateur nécessitent le pont Python KOKURA et sa DLL C API. Pillow est facultatif pour les PNG.

<!-- wire3d-feedback:end -->
