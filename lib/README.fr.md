# Bibliothèques KITAQGB

`wire3d_dmg` est une bibliothèque de rendu filaire monochrome pour Game Boy. Compilez `wire3d_dmg_96.c` pour une vue de 128 × 96 ou `wire3d_dmg.c` pour 128 × 120, puis utilisez les fonctions `Wire3DDMG_*`. `wire3d` et `dmg3d` offrent aussi des points d’entrée pour les profils de 96 et 120 lignes, respectivement. Compilez un seul point d’entrée par programme. `wire3d_cgb` est le moteur dédié au rendu couleur.

[Guide du moteur commun (anglais)](wire3d_dmg_guide.md) / [日本語](wire3d_dmg_guide_ja.md)

[English](README.md) | **Français**

**[Ouvrir le manuel de la bibliothèque KITAQGB](https://bartaro.github.io/kitaq-docs/fr/gb-library.html)**

Ce dossier contient trois catégories de fichiers :

- **API publiques** : bibliothèques réutilisables à inclure et à compiler avec le jeu.
- **Unités auxiliaires** : composants facultatifs, déclarations de registres ou unités vides qui ne constituent pas, à elles seules, une API publique indépendante.
- **Documents de référence** : documentation qui n'est pas liée aux ROM.

## Classement

### API publiques

| Fichiers | Rôle | Utilisation habituelle |
| --- | --- | --- |
| `physics2d.h` / `physics2d.c` | Corps AABB 2D, intégration de la gravité et résolution itérative des contacts. | Inclure l'en-tête et compiler le source utilisé. |
| `physics2d_circle.h` / `physics2d_circle.c` | Physique de corps circulaires 2D pour les jeux de balles. | Inclure l'en-tête et compiler le source utilisé. |
| `physics3d.h` / `physics3d.c` | Physique AABB 3D, accélération, rebonds pondérés par la masse, indicateurs de rupture et `kq3d_dot_q8_8()`. | Inclure l'en-tête et compiler le source utilisé. |
| `wire3d.h` / `wire3d.c` | Rendu filaire 3D en virgule fixe, tampon WRAM, modèles à lignes cachées et masques d'occultation de scène. | Inclure `wire3d.h` et compiler `wire3d.c`. |
| `dmg3d.h` / `dmg3d.c` | Rendu filaire DMG monochrome : surface de 128 × 120 en D000, tracé assembleur à ordre fixe et transfert de D000 vers 8900 contrôlé par STAT. | Inclure et compiler ces fichiers pour un rendu filaire à 1 bit par pixel. |
| `wire3d_cgb.h` / `wire3d_cgb.c` | Rendu filaire couleur réservé à la CGB à 8 MHz : tampon WRAM 2bpp, occultation, tracé assembleur avec découpage et présentation sans déchirure par DMA HBlank. | Inclure l'en-tête et compiler le source dans une ROM réservée à la CGB. |
| `system.h` / `system.c` | Base d'exécution GB : initialisation, compteur d'images, attente VBlank, rappel coopératif et fonctions DI/EI. | Inclure et compiler pour une boucle rythmée par les images. |
| `input.h` / `input.c` | État des commandes par image : maintien, appui, relâchement et répétition. | Inclure et compiler pour les menus, jeux d'action, puzzles et jeux de stratégie. |
| `vram.h` / `vram.c` | File de commandes VRAM respectant le PPU : tuiles BG, rectangles, blocs de carte, copies et remplissages mémoire. | Préparer les mises à jour pendant le jeu, puis appeler `vram_flush()` ou `vram_flush_now()` au moment approprié. |
| `sprite.h` / `sprite.c` | Copie de travail de l'OAM, allocation, métasprites, animation, DMA et détection des dépassements par ligne. | Inclure et compiler pour le rendu par OBJ. |
| `fixed.h` / `fixed.c` | Calcul Q8.8, `Vec2`, `KQRect`, bornage, minimum, maximum, interpolation et tests de rectangles. | Inclure et compiler pour le mouvement, la physique, la caméra et les évaluations d'IA. |
| `scene.h` / `scene.c` | Table légère de scènes de type titre/jeu/pause et répartition des changements, mises à jour et dessins. | Inclure et compiler pour organiser les états du jeu. |
| `entity.h` / `entity.c` | Réserve en tableau fixe de jusqu'à `ENTITY_MAX` petites entités. | Inclure et compiler ; les rappels reçoivent un identifiant et peuvent appeler `entity_get(id)`. |
| `danmaku.h` / `danmaku.c` | Réserve de 96 projectiles en virgule fixe, éventails sur 32 directions, impacts, frôlements et composition de tuiles BG CGB indépendante des limites OAM. | Inclure et compiler ; consulter `danmaku_guide.md`. Le jeu de référence `ressen_gbc` appartient aux exemples de développement historiques. |
| `bank.h` / `bank.c` | Accès lointains aux données et fonctions, pointeurs lointains et commutation MBC simple sur les fonctions intrinsèques. | Inclure et compiler pour les accès aux données en banques. |
| `asset.h` / `asset.c` | Petite table de descripteurs par identifiant et chargement de données brutes ou de tuiles. | Inclure et compiler ; des fichiers `assets.h/c/json` générés peuvent ensuite viser cette organisation. |
| `debug.h` / `debug.c` | Petit tampon ROM de traces, assertions et marqueurs pour l'observation par KOKURA ou un émulateur. | Inclure et compiler ; conserver le profilage lourd hors de la ROM. |
| `chain.h` / `chain.c` | Historique circulaire de coordonnées pour serpent, corde, train ou sprites articulés. | Inclure et compiler pour le déplacement de segments suivant un historique. |
| `cgb_tile.h` | Déclarations des fonctions intrinsèques CGB de tuiles et d'attributs. | Inclure pour utiliser `__settile...` et les fonctions de tuiles CGB. |
| `cgb_palette.h` / `cgb_palette.c` | Gestion de haut niveau des palettes BG/OBJ CGB. | Inclure l'en-tête et compiler le source utilisé. |
| `scroll.h` / `scroll.c` | Défilement et tables de fractionnement fondés sur les fonctions intrinsèques. | Inclure et compiler pour les fonctions `Scroll_*`. |
| `raster.h` / `raster.c` | Construction de bandes de défilement et profils structurés de déformation horizontale par ligne. | Inclure l'en-tête et compiler avec `scroll.c` pour les fonctions `Raster_*`. |
| `camera.h` / `camera.c` | Caméra en virgule fixe 8.8 fondée sur `scroll.*`, fonctions de caméra globale et conversion monde/écran. | Inclure l'en-tête et compiler le source utilisé. |
| `audio.h` / `audio.c` | Pilote GB commun : musique, effets, panoramique, ondes et fondus, avec 68 indices de notes jusqu'à 67. | Inclure et compiler pour les projets sonores. Les huit derniers indices reprennent actuellement les fréquences de l'octave précédente. |
| `audio_vblank.h` / `audio_vblank.c` | Musique par interruption VBlank, mêmes indices de notes, file WRAM de 16 événements pour morceaux en banques et rappel par image facultatif. | Garder les morceaux directement pointés en banque fixe 0, ou alimenter la file depuis le code en banques ; raccorder le vecteur 0x0040. |
| `link.h` / `link.c` | Transferts série d'octets et fonctions logiques coopératives `Link4_*`. | Inclure et compiler pour les projets avec liaison. |
| `link_packet.c` | Couche de paquets facultative sur `link.c`, avec boîtes de réception `Link4_*` par correspondant. | Compiler avec `link.c` si l'envoi ou la lecture de paquets est nécessaire. |
| `link_dmg07.h` / `link_dmg07.c` | Pilote par interrogation et horloge externe de l'adaptateur physique Nintendo DMG-07. | Compiler avec `link_hwregs_gb.c` ; API distincte de `Link4_*`. |
| `rpg.h` | Déclarations RPG / ADV / SLG et fonctions intrinsèques de bas niveau. | Inclure pour utiliser cette famille de fonctions. |
| `rng.c` | `rng8`, `rng16`, `rand_range`, `weighted_choice`, `rng_seed`, `rng_next8`, `rng_next16`, `rng_range` et `rng_chance`. | Compiler pour les fonctions aléatoires déclarées dans `rpg.h`. |
| `flags.c` | Ensemble de 2 048 indicateurs binaires et états de quêtes. | Compiler pour les indicateurs et quêtes de `rpg.h`. |
| `rle.c` | Décodage RLE simple `[count][value]` depuis la RAM ou la ROM distante. | Compiler pour les fonctions `rle_decode*` de `rpg.h`. |
| `text.c` | Fenêtres de chaînes de tuiles, attente entre pages, choix, texte XY, nombres et alias de fenêtre/effacement. | Compiler pour les fonctions de texte de `rpg.h`. |
| `menu.c` | Menus verticaux, inventaire minimal et API de menu non bloquante. | Compiler pour les menus de `rpg.h`. |
| `script.c` | Interpréteur minimal de bytecode pour les séquences RPG / ADV. | Compiler pour les scripts de `rpg.h`. |
| `map.c` | Chargement de cartes compactes, collisions, déclencheurs, caméra et métatuiles 16 × 16 facultatives. | Compiler pour les cartes de `rpg.h`. |
| `save.c` | Sauvegarde SRAM de type MBC5 : lecture, écriture, contrôle et effacement, avec en-tête, version, longueur et somme de contrôle. | Compiler pour les sauvegardes de `rpg.h`. |
| `slg_unit.c` | Portées de déplacement et d'attaque tactiques. | Compiler pour les unités tactiques de `rpg.h`. |
| `slg_path.c` | Recherche de chemin en largeur et propagation des coûts de déplacement. | Compiler pour les chemins tactiques de `rpg.h`. |
| `slg.h` / `slg_board.c` | Plateaux génériques, listes de coups et pile d'annulation pour jeux de plateau ou tactiques. | Inclure et compiler ; garder l'évaluation propre au jeu en dehors de ce composant. |

### Unités auxiliaires

| Fichiers | Rôle | Remarque |
| --- | --- | --- |
| `audio_hwregs_gb.c` | Déclarations minimales des registres APU et de la RAM d'onde. | À utiliser seulement si le projet ne les définit pas déjà ailleurs. |
| `link_hwregs_gb.c` | Déclarations minimales de `SB`, `SC`, `IF` et `IE`. | À utiliser seulement si les registres série ne sont pas déjà définis ailleurs. |
| `cgb_tile.c` | Unité volontairement vide de la famille des tuiles CGB. | L'interface réelle est dans `cgb_tile.h` ; compiler ce fichier est sans effet utile et facultatif. |
| `math.c` | Table sinus en ROM `MATH_SIN`. | Présente dans `lib/`, mais pas encore documentée comme API publique stable ; à considérer comme unité de données auxiliaire. |

### Documents de référence

| Fichier | Contenu |
| --- | --- |
| `README.md` | Présentation et indications de compilation. |
| `wire3d_guide_ja.md` | Démarrage rapide en japonais du rendu filaire 3D. |
| `dmg3d_guide_ja.md` | Démarrage rapide en japonais du rendu DMG monochrome. |
| `wire3d_cgb_guide.md` | Démarrage rapide du rendu filaire couleur réservé à la CGB. |
| `physics_guide.html` | Guide de physique en anglais. |
| `physics_guide_ja.html` | Guide de physique en japonais. |

## Utilisation à la compilation

Les commandes montrent comment compiler les sources d’une application avec les modules de la bibliothèque. Préparez les fichiers de l’application indiqués dans chaque commande. Pour les programmes d’initiation fournis, utilisez `../examples/build.ps1` et le manuel HTML. La commande abrégée `kitaqgb` suppose que l’exécutable se trouve dans le PATH.

```powershell
kitaqgb hwregs.c lib/audio.c main.c lib/physics2d.c lib/physics2d_circle.c lib/physics3d.c lib/cgb_palette.c lib/scroll.c lib/camera.c -I lib -o game.gb --profile=dev
```

Pour le rendu filaire 3D, compilez le moteur avec le source du jeu :

```powershell
.\kitaqgb.exe lib/wire3d.c examples/wire3d_minimal.c -I lib -o examples/wire3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

Le programme fourni `examples/wire3d_minimal.c` initialise tous les champs du modèle et fait tourner un cube dans la vue de 128 × 96. Il ne nécessite ni ressource graphique externe ni police.

Pour un rendu DMG monochrome, utilisez l’entrée de compatibilité à 120 lignes `dmg3d.*` :

```powershell
.\kitaqgb.exe lib/dmg3d.c examples/dmg3d_minimal.c -I lib -o examples/dmg3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

`DMG3D_Init()` configure une vue de 128 × 120 avec un tampon WRAM en D000 et des transferts de tuiles à partir de 0x8900. `DMG3D_BeginFrame()` ne réinitialise que l’occultation ; les transferts consomment et effacent les pixels. `DMG3D_EndFrame()` attend VBlank, puis contrôle STAT pendant la copie, qui peut se poursuivre après VBlank. Le programme fourni `examples/dmg3d_minimal.c` redessine une croix à chaque image avec le transfert des tuiles modifiées activé. Le transfert auxiliaire partage sa mémoire source avec le tampon principal.

Le rendu filaire couleur réservé à la CGB utilise `wire3d_cgb.*` :

```powershell
kitaqgb lib/wire3d_cgb.c examples/wire3d_cgb_color_demo.c -I lib -o examples/wire3d_cgb_color_demo.gbc --profile=dev --stack-bank=fixed --rst-disable --cgb=cgb_only --rom-title=CGBWIRE3D
```

`Wire3DCGB_Init()` passe la CGB en double vitesse, configure une surface filaire BG de 128 × 96 en 2bpp et installe la palette BG de quatre couleurs par défaut. Les deux paires d'API d'image utilisent une présentation sans déchirure : le tampon de 3 072 octets à `0xD300–0xDEFF` est envoyé par DMA HBlank vers la banque de tuiles VRAM inactive, puis affiché pendant VBlank sans changement de LCDC à chaque image. La paire `Fast` omet le contrôle habituel de la file BG en fin d'image. Utilisez `Wire3DCGB_SetPaletteRGB15()` et `Wire3DCGB_SetLineColor()` / `Wire3DCGB_Draw*Color()` pour tracer des lignes colorées.

Pour les niveaux de détail directionnels générés par CAO, `Wire3DCGB_DrawMaskedModel2D()` reçoit des décalages de sommets signés déjà projetés et un masque compact d'arêtes visibles. Le parcours des arêtes et le rasteriseur assembleur restent dans la banque 4 du moteur ; dessiner un modèle n'impose donc pas un appel entre banques pour chaque ligne.

Un jeu qui ne modifie que peu de zones et utilise une copie de travail de l'OAM peut appeler `sprite_flush_oam()`, puis `Wire3DCGB_EndFrameSparseNow()` après être entré dans VBlank. Cela évite une attente initiale du prochain VBlank, mais le DMA et la présentation peuvent encore attendre selon la zone modifiée et la ligne de balayage en cours. Rien ne garantit que tout se termine pendant ce même VBlank.

Pour les lignes CGB, utilisez les couleurs 1, 2 et 3. Le mode normal de 128 × 96 combine les bits de couleur : la superposition des couleurs 1 et 2 donne la couleur 3. La couleur 0 n'efface pas une ligne. Effacez l'image ou utilisez les fonctions d'effacement prévues à cet effet. En mode normal, `Wire3DCGB_DrawLine2D` et le dessin des modèles ne mémorisent pas la zone à transférer. Pour tracer des lignes en enregistrant cette zone, utilisez `Wire3DCGB_DrawLineClipped2D` ; pour inclure toute la zone de dessin dans le prochain transfert partiel, appelez `Wire3DCGB_InvalidateFrameHistory`.

Le mode de 160 × 144 alloue au maximum 127 tuiles par image. Si le tracé rapide ne peut pas allouer une tuile ou rencontre une coordonnée hors écran, `Wire3DCGB_GetFullScreenOverflow()` renvoie une valeur non nulle et les écritures de pixels sont suspendues jusqu'à l'initialisation de l'image suivante. Gardez les sommets dans la zone sélectionnée. La marge droite du masque triangulaire s'arrête à X=127 en mode 128 × 96 et à X=159 en plein écran. Respectez les exigences de sélection des banques WRAM indiquées pour chaque API, en particulier avec le plein écran ou FastMap.

Le [test des limites du masque triangulaire CGB](../tests/library/wire3d_cgb_mask_bounds.c) est un programme complet qui vérifie les deux modes d'affichage.

Exemples historiques de compilation RPG / ADV / SLG :

```powershell
kitaqgb examples/example_rpg_text.c lib/text.c lib/menu.c -I lib -o text.gb --profile=dev
kitaqgb examples/example_adv_script.c lib/text.c lib/flags.c lib/script.c -I lib -o script.gb --profile=dev
kitaqgb examples/example_slg_cursor.c lib/map.c lib/slg_unit.c lib/slg_path.c -I lib -o slg.gb --profile=dev
```

Compilation du test élémentaire des services d'exécution :

```powershell
kitaqgb lib/text.c lib/menu.c lib/map.c lib/scroll.c lib/camera.c lib/rng.c lib/save.c lib/system.c lib/input.c lib/vram.c lib/sprite.c lib/fixed.c lib/scene.c lib/entity.c lib/bank.c lib/asset.c lib/debug.c lib/chain.c lib/physics2d.c lib/slg_board.c examples/standard_library_smoke.c -I lib -o examples/standard_library_smoke.gb --profile=dev --rom-title=STDLIBSMK --no-disasm
```

Pour la liaison série, placez l'unité de registres avant le cœur de liaison :

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c main.c -I lib -o game.gb --profile=dev
```

Les projets logiques coopératifs à quatre joueurs utilisent les mêmes fichiers. L'hôte appelle `Link4_InitHost(slot_count)` et sélectionne le correspondant par `Link4_SelectPeer()` ou `Link4_SendPacketTo()`. Les correspondants appellent `Link4_InitPeer(local_slot, slot_count)` et communiquent avec l'emplacement hôte `0`.

Les exemples historiques de correspondants peuvent être compilés avec les enveloppes d'emplacement préparées :

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot1.c -I lib -o peer1.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot2.c -I lib -o peer2.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot3.c -I lib -o peer3.gb --profile=dev
```

Les projets pour l'adaptateur physique DMG-07 utilisent à la place le pilote par interrogation :

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link_dmg07.c main.c -I lib -o dmg07.gb --profile=dev
```

Appelez `LinkDmg07_Poll()` en continu : les octets de l'adaptateur sont beaucoup plus rapprochés qu'une image vidéo. Appelez `LinkDmg07_TickFrame()` une fois par VBlank pour les compteurs saturants de silence et de délai de négociation. Le pilote arme toujours l'horloge externe avec `SC=$80`, répond aux interrogations par `88 88 RATE 01` et permet uniquement au joueur physique 1 de demander la transmission avec `AA AA AA AA`. Après observation de `CC CC CC CC` par toutes les consoles, chaque diffusion de quatre octets contient un octet par emplacement physique. L'adaptateur diffuse les données un paquet après leur soumission ; le pilote ignore donc le premier paquet indéfini et expose les numéros de séquence d'émission et de réception.

`LinkDmg07_RequestRestart()` attend la limite du paquet suivant, envoie un `FF FF FF FF` aligné et s'arrête dès réception de l'indicateur complet composé uniquement de FF. Un dépassement du délai de silence pendant le transfert programme automatiquement ce redémarrage sans perdre la position actuelle dans les quatre octets. Si les impulsions d'horloge reprennent, elles peuvent ainsi terminer le paquet courant et engager correctement la récupération.

Incluez ensuite les en-têtes nécessaires depuis votre jeu :

```c
#include "physics2d.h"
#include "physics2d_circle.h"
#include "physics3d.h"
#include "wire3d.h"
#include "cgb_tile.h"
#include "cgb_palette.h"
#include "scroll.h"
#include "raster.h"
#include "camera.h"
#include "audio.h"
#include "audio_vblank.h"
#include "system.h"
#include "input.h"
#include "vram.h"
#include "sprite.h"
#include "fixed.h"
#include "scene.h"
#include "entity.h"
#include "bank.h"
#include "asset.h"
#include "debug.h"
#include "chain.h"
#include "slg.h"
```

## Remarques

- `inv_mass_q8 == 0` indique un corps statique.
- `Wire3D_Init()` réserve une surface filaire BG de 128 × 96, un tampon WRAM à partir de `0xD000` et des tuiles à partir de `0x8900`. Compilez avec `--stack-bank=fixed`.
- `Wire3D_BeginFrame()` efface le tampon WRAM ; `Wire3D_EndFrame()` attend VBlank puis effectue une copie en rafales contrôlée par STAT.
- Les angles Wire3D ont 16 positions ; le premier chemin de rendu des modèles accepte jusqu'à `WIRE3D_MODEL_VERTEX_LIMIT` sommets par modèle.
- Utilisez `Wire3D_DrawScene()` lorsque plusieurs objets filaires d'allure solide se superposent : les objets proches sont dessinés d'abord, puis leurs masques de faces visibles permettent d'écarter prudemment les lignes plus lointaines.
- `Wire3DCGB_Init()` est réservé à la CGB et passe en double vitesse par KEY1/STOP. Compilez avec `--cgb=cgb_only` et ne combinez pas `wire3d_cgb.*` et le moteur compatible DMG `wire3d.*` dans une même ROM.
- Les paires d'image standard et `Fast` gardent l'ancienne image visible jusqu'à la fin de la banque VRAM inactive. Préférez `Fast` si la scène n'utilise pas d'écritures de tuiles BG en file.
- Compilez les déclarations `NR10..NR52` et `WAVE0..WAVE15` avant `lib/audio.c`.
- `lib/audio_hwregs_gb.c` fournit ces déclarations, mais ne doit pas être compilé avec une autre unité qui définit déjà les mêmes registres.
- `cgb_tile.h` expose directement les fonctions intrinsèques ; `lib/cgb_tile.c` est une unité vide facultative.
- L'API publique de `cgb_palette.h` utilise les noms `cgb_*`.
- Appelez `Audio_SetMusicEnabled()` / `Audio_SetSfxEnabled()` lorsque les menus ou réglages changent.
- `Audio_PlaySFX()` mémorise la banque ROM visible. Utilisez `Audio_PlaySFXBanked(bank, sfx, priority)` si la banque des données est connue explicitement.
- `AUDIO_CMD_NOTE` / `AUDIO_CMD_SET_INST` conservent les identifiants historiques : `0=CH1`, `1=CH2`, `2=CH4`, `3=CH3`.
- `Audio_LoadCustomWave()` reçoit 16 octets contenant 32 échantillons de 4 bits pour une forme d'onde CH3 personnalisée.
- `Audio_FadeToMasterVolume()` progresse dans `Audio_Update()` ; continuez les appels à chaque image pendant un fondu.
- `audio_vblank.c` définit `__kq_vblank_vector`. Ses événements musicaux font cinq octets : `delay, ch2_note, ch1_note, ch3_note, ch4_noise_param`. Utilisez `AUDIO_VBLANK_REST`, `AUDIO_VBLANK_LOOP` et `AUDIO_VBLANK_END`.
- Les morceaux directement pointés doivent être en banque fixe ; le mode file permet d'alimenter des morceaux en banques. Après l'édition de liens, `scripts/patch_gb_vblank_irq.ps1 <rom> <map>` raccorde le vecteur `0x0040` et actualise les sommes de contrôle. Le script fourni se trouve dans `scripts` ; consultez le manuel pour l'intégration du pilote.
- Ne combinez pas `audio_vblank.c` avec une autre unité possédant le vecteur VBlank `0x0040` sans répartiteur commun.
- `Scroll_SplitCommit()` active automatiquement les bits IE `0x01 | 0x02` et utilise les gestionnaires VBlank/STAT du compilateur.
- Le fractionnement réserve actuellement `0x0040` et `0x0048` ; ne le combinez pas avec un autre gestionnaire personnalisé de ces vecteurs.
- Compilez les déclarations `SB`, `SC`, `IF` et `IE` avant `lib/link.c` / `lib/link_packet.c` ou `lib/link_dmg07.c`.
- `lib/link_hwregs_gb.c` fournit ces déclarations ; ne les dupliquez pas dans une autre unité.
- La bibliothèque de liaison ne possède pas le vecteur `0x0058` ; en mode interruption, appelez `Link_OnSerialIRQ()` depuis votre gestionnaire ou répartiteur.
- La couche de paquets conserve volontairement un seul paquet en attente et se pilote de préférence depuis une boucle rythmée par les images.
- `Link4_*` modélise un adaptateur logique coopératif : un seul correspondant utilise la liaison à la fois et l'hôte doit les sélectionner à tour de rôle.
- `Link4_TryReadByteFrom()` / `Link4_HasPacketFrom()` exposent des boîtes par correspondant pour conserver l'origine des données.
- `Link_ReadPacket()` renvoie le paquet le plus récent ; utilisez `Link4_ReadPacketFrom()` pour quatre joueurs.
- `Link4_*` n'implémente pas le comportement électrique ou le protocole du DMG-07. Utilisez `link_dmg07.c` pour l'accessoire physique et ne le compilez pas avec `link.c` dans la même ROM.
- Le masque DMG-07 `GetConnectedMask()` utilise les bits 0 à 3 pour les joueurs physiques 1 à 4. Pendant le transfert, il conserve le dernier résultat d'interrogation ; la liste des participants n'est actualisée qu'en phase d'interrogation.
- Un redémarrage DMG-07 en attente ne progresse pas sans horloge de l'adaptateur. Le trafic de récupération est écarté au lieu d'être exposé comme données du séquenceur. Si l'accessoire a été éteint puis rallumé dans une autre phase, réinitialisez explicitement le pilote ou la session.
- Les bibliothèques physiques résolvent les positions et vitesses linéaires, pas la dynamique angulaire.
- Gardez peu de corps actifs sur une machine de type GB, par exemple 8 à 24.
- Ajustez gravité, vitesse maximale et nombre d'itérations de résolution pour chaque monde selon le jeu.
- Pour un jeu de type billard, préférez `physics2d_circle.*` à la bibliothèque AABB.
- Dans l'organisation actuelle, ne créez pas de bibliothèques séparées `random`, `collision`, `ui`, `tilemap`, `dialog`, `board_game` ou `simple_physics`. Utilisez respectivement `rng`, `physics2d`, `text`/`menu`, `map`, `script`, `slg` et `physics2d`.
- `scene.c` et `entity.c` évitent les arguments de rappel de taille pointeur : le chemin d'appel actuel de KITAQGB est surtout adapté aux rappels sans argument ou recevant un identifiant sur un octet.

Pour la file de transfert VRAM, `vram_get_queue_capacity()` donne la capacité totale en emplacements de commande, `vram_get_queue_free()` le nombre libre et `vram_get_queue_used()` le nombre occupé. Il ne s'agit pas d'une mesure de l'espace libre de la VRAM matérielle.
