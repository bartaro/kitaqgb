# Original LINK16 export templates

GBSEQ embeds these 512 KiB MBC5 templates at build time. They contain the
original KITAQGB sequencer/player runtime, not imported compositions. GBSEQ
writes the project data into reserved ROM banks during export.

- `link16_player_{cable,dmg07}.gb`: dedicated per-part player for GB/GBC;
  cartridge RAM size zero. The title graphics are generated on export.
- `link16_export_{cable,dmg07}.gb`: editable song ROM, requiring 128 KiB
  cartridge SRAM. Do not use it in place of a no-SRAM solo player.

The included ROM text glyphs retain their [MIT notice](../../licenses/assets/ASCII-font-MIT.txt).
See [player ROM format](../../docs/LINK16_PLAYER_ROM_FORMAT.md) for the manifest and part layout.

The templates and exported ROMs retain the 48-byte Nintendo logo data in
their cartridge headers for boot-time compatibility. GBSEQ's MIT license
does not grant independent rights to Nintendo's logo or trademarks; GBSEQ
is not affiliated with or approved by Nintendo. See the
[English/Japanese scope notice](../../docs/NINTENDO_NOTICE.md).
