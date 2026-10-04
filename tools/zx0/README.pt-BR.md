# Ferramentas auxiliares nativas

[en](../README.en.md) · [ja](../README.ja.md) · [ko](../README.ko.md) · [zh-CN](../README.zh-CN.md) · [zh-TW](../README.zh-TW.md) · [fr](../README.fr.md) · [es](../README.es.md) · [de](../README.de.md)

Todas as ferramentas são implementadas em Rust e executam sem .NET, Python ou Pillow. Compile todos os executáveis com `cargo build --locked --release`. No Windows, acrescente `.exe` aos comandos.

```text
kitaqgb-zx0 input.bin output.zx0
kitaqgb-zx0 output.zx0 restored.bin --decompress
kitaqgb-zx0 input.bin asset.h --header=level_data
kitaqgb-zx0 input.bin output.kqa --format=auto
kitaqgb-patch-vblank --rom game.gb --map game.map
```

ZX0 aceita de 1 a 65535 bytes e preserva a saída do codificador C#. Suporta `raw`, `rle` contagem/valor e o contêiner automático `KQA1` de nove bytes. Em empate, prefere raw, RLE e ZX0. `--decompress` decodifica um fluxo ZX0 v2 direto com limite de saída. Fluxos reversos e v1 não são suportados. Einar Saukas criou o formato; esta implementação KITAQ usa a licença MIT.

O corretor VBlank verifica o símbolo do banco fixo e a assinatura PUSH, altera a ROM diretamente e recalcula as somas. `--no-header-fix` mantém as somas originais. A busca não valida toda a rotina. Faça uma cópia.

```sh
sh tools/zx0/build.sh
```
