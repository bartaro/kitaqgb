# HARAPEKO SHIROHEBI

<!-- readme-language-links:start -->
[English](README.md#english) | [日本語](README.md#%E6%97%A5%E6%9C%AC%E8%AA%9E) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | **Français** | [Español](README.es.md) | [Deutsch](README.de.md)
<!-- readme-language-links:end -->

Un jeu de course au meilleur score pour Game Boy / Game Boy Color, créé par **DAISUKE OBA**. Dirigez le serpent blanc, mangez des grenades pour grandir et évitez les mines ainsi que votre propre corps.

- **Présentation et téléchargement officiel de la ROM :** <https://bartaro.itch.io/harapeko-shirohebi>
- **Guide de programmation :** [HTML en français](https://bartaro.github.io/kitaq-docs/apps/harapeko_shirohebi/guide-fr.html)
- **Licence :** [MIT](LICENSE), copyright © 2026 DAISUKE OBA. Elle couvre le code du jeu, les graphismes originaux fournis, les données de caractères, la musique, les effets sonores et la documentation de ce répertoire. Conservez la mention de licence lors de leur redistribution. KITAQGB et ses dépendances gardent les mentions de la [licence du dépôt](../../LICENSE), dont la [mention de la police ASCII originale](../../licenses/fonts/ASCII-font-MIT.txt).

Le guide HTML contient un schéma d’exécution, l’algorithme de suivi du serpent, des exemples d’utilisation des bibliothèques et un repérage des fichiers sources. Le lien ci-dessus ouvre directement la page KITAQ Docs dans le navigateur. Le HTML et la feuille de style sont maintenus dans le [dépôt kitaq-docs](https://github.com/bartaro/kitaq-docs/tree/main/apps/harapeko_shirohebi).

## Compiler sous Windows

Il faut Windows, .NET Framework 4.8, PowerShell et une copie complète du dépôt, obtenue par Git ou sous forme de ZIP. Conservez ensemble les fichiers du compilateur à la racine et `lib/`. Les graphismes et l’audio sont fournis sous forme de tableaux C ; aucun éditeur de ressources n’est nécessaire.

Depuis la racine du dépôt :

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\apps\harapeko_shirohebi\build.ps1
```

Ou depuis le répertoire du jeu :

```powershell
.\build.ps1
# If the game is stored separately, select a complete KITAQGB installation:
.\build.ps1 -KitaqgbRoot C:\tools\kitaqgb
# Retain compiler intermediates for debugging:
.\build.ps1 -KeepBuildFiles
```

La sortie par défaut est `out/shirohebi.gb`, accompagnée de `out/shirohebi.map` et de `out/build_manifest.json`. Utilisez `-OutputDirectory C:\build\shirohebi` pour choisir une autre destination. Une compilation réussie supprime son répertoire temporaire, sauf si `-KeepBuildFiles` est défini ; un échec le conserve pour le diagnostic. Les fichiers produits sont ignorés par Git.

Le script assemble les fragments du jeu, compile avec les bibliothèques du dépôt, vérifie que les routines d’interruption sont en ROM fixe, installe le vecteur VBlank et met à jour les sommes de contrôle du cartouche. Il produit une **ROM MBC5 de 64 KiB avec 8 KiB de RAM sauvegardée par pile**, jouable en modes DMG et CGB. Ne compilez pas les fragments séparément et ne sautez pas l’étape du vecteur et des sommes de contrôle.

Ce répertoire d’application ne contient **aucun exécutable ni aucune ROM précompilés**. Le compilateur est fourni à la racine du dépôt ; la ROM publiée du jeu se télécharge sur itch.io.

## Commandes

| Écran | Action |
|---|---|
| Titre | START : commencer. UP/DOWN/SELECT : choisir MUSIC ou SOUND. LEFT/RIGHT/A : activer ou désactiver l’option choisie. |
| Jeu | LEFT/RIGHT : tourner par rapport à l’orientation du serpent. Maintenir UP : accélérer. START : pause. |
| Pause | START : reprendre. SELECT : ouvrir la confirmation de retour au titre. |
| Confirmation | LEFT/RIGHT/SELECT : choisir. A : confirmer. B/START : annuler. |
| Saisie du nom | UP/DOWN : choisir A–Z ou un point. LEFT/RIGHT/SELECT : déplacer le curseur. A : avancer, ou terminer au troisième caractère. START : terminer. |
| Nouvelle partie | LEFT/RIGHT/SELECT : choisir YES/NO. A/START : confirmer. |

Sur le titre, SELECT+START ouvre la confirmation d’effacement des scores. Après un changement d’écran, tous les boutons doivent être relâchés avant qu’une nouvelle commande soit acceptée. Consultez le guide HTML pour les transitions d’état et les détails de l’implémentation.
