# Compression de ressources compatible ZX0

<!-- readme-language-links:start -->
[English](README.md#english) | [日本語](README.md#%E6%97%A5%E6%9C%AC%E8%AA%9E) | [한국어](README.ko.md) | [繁體中文](README.zh-TW.md) | **Français** | [Español](README.es.md) | [Deutsch](README.de.md)
<!-- readme-language-links:end -->

[API et exemples](https://bartaro.github.io/kitaq-docs/fr/gb-library.html#module-zx0)

Le compresseur PC et le décompresseur GB sont des implémentations indépendantes de KITAQ pour les flux ZX0 v2 en lecture avant. L’implémentation KITAQ est distribuée sous licence MIT, copyright (c) 2026 DAISUKE OBA.

Le format ZX0 et l’algorithme de compression d’origine ont été conçus par [Einar Saukas](https://github.com/einar-saukas/ZX0). Cette reconnaissance du format est distincte du droit d’auteur et de la licence de l’implémentation KITAQ. Voir [LICENSE](../../LICENSE) et [LICENSE.ja](../../LICENSE.ja).

Compilez l’outil PC depuis la racine du dépôt :

```powershell
.\tools\zx0\build.ps1
.\kitaqgb-zx0.exe input.bin output.zx0
.\kitaqgb-zx0.exe input.bin asset.h --header=level_data
.\kitaqgb-zx0.exe output.zx0 restored.bin --decompress
```

L’outil autonome utilise .NET Framework 4.x et accepte de 1 à 65535 octets en entrée. Il effectue une recherche bornée par chaîne de hachage, sans garantir la taille compressée optimale. Il produit des données ZX0 v2 ordinaires, sans enveloppe KITAQ. Les flux inverses, les dictionnaires de préfixes et ZX0 v1 ne font pas partie de cette interface. Les droits sur les ressources restent à leurs auteurs.

Toute sortie encodée doit tenir dans le paramètre de taille de 65535 octets de l’API cible. Pour les conteneurs automatiques, l’en-tête de neuf octets compte dans cette limite. Découpez les ressources trop grandes et respectez aussi la RAM et les fenêtres de banques, bien plus petites sur la cible. Un en-tête C raw vide contient un octet de stockage réservé, avec une taille logique `_SIZE` de zéro. Un flux ZX0 brut vide n’est pas pris en charge.

Pour comparer les données brutes, le RLE par couples nombre/valeur et ZX0, puis conserver la plus petite charge utile :

```powershell
.\kitaqgb-zx0.exe input.bin output.kqa --format=auto
```

Le mode automatique ajoute un en-tête KQA1 de neuf octets et indique le codec choisi. Il compare les charges utiles ; en cas d’égalité, l’ordre de préférence est raw, RLE, puis ZX0. Il s’agit d’un conteneur de ressources KITAQ, pas d’un flux ZX0 brut. L’en-tête contient `KQA1`, un octet de codec (0 raw, 1 RLE, 2 ZX0), la taille originale en u16 petit-boutiste, puis la taille de la charge utile au même format. Le corps suit immédiatement. Utilisez `asset_decompress` pour ce conteneur. `--format=raw` et `--format=rle` produisent uniquement la charge utile choisie ; un nombre nul termine le RLE.

Incluez `zx0.h` et compilez `lib/zx0.c` pour la cible. `zx0_decompress` reçoit la destination, sa capacité, la source compressée et sa taille. Vérifiez à la fois le nombre d’octets renvoyé et `zx0_error`. Une erreur peut laisser une sortie partielle : ne l’affichez pas et ne l’utilisez pas après un échec. Les tampons source et destination ne doivent ni se chevaucher ni traverser les fenêtres de banques CPU actuellement mappées. Ces fonctions partagent une zone de travail et ne doivent pas être rappelées depuis une interruption pendant leur exécution.

Les tampons ne doivent pas non plus déborder de l’espace d’adressage CPU et revenir au début.

`zx0_decompress_vram` écrit dans la banque VRAM GB sélectionnée uniquement lorsque le LCD est éteint. Il conserve les réglages d’affichage, de banque et d’interruption. Effectuez le transfert lors de l’initialisation d’une scène ; ne supposez pas qu’une ressource entière tient dans un seul VBlank.
