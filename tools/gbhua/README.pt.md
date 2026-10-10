# GBHUA

[English](README.en.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Español](README.es.md) | [Português](README.pt.md) | [Français](README.fr.md) | [Deutsch](README.de.md)

O GBHUA converte PNG em tiles, paletas e mapas para Game Boy. Autor: DAISUKE OBA. Código original: MIT.

## Compilar e executar

```sh
cd tools/gbhua
cargo build --release --locked
./target/release/gbhua --help
```

Windows: `target\release\gbhua.exe`. Rust >= 1.92.

## Executáveis para macOS

CLI para Apple Silicon e Intel: [downloads e início](MACOS.md).
Extraia o arquivo correto, abra o Terminal nessa pasta e execute `./gbhua --help`.
Nos exemplos, use `./gbhua` em vez de `./target/release/gbhua`. Não precisa de Rust nem Python.
Alvo mínimo de compilação: macOS 11.0; testado no macOS 15. Sem assinatura Apple Developer ID ou notarização.

## Fluxo de trabalho

Importe o PNG, consulte o relatório JSON compacto, valide, visualize e exporte. Cada comando bem-sucedido emite um objeto JSON. Para reduzir tokens, forneça à IA o caminho e o relatório, não o projeto inteiro.

```sh
./target/release/gbhua import-image artwork.png --out scene.gbh --mode cgb --size 160x144
./target/release/gbhua inspect scene.gbh
./target/release/gbhua validate scene.gbh
./target/release/gbhua preview scene.gbh --out preview.png --mode cgb --scale 2
./target/release/gbhua export scene.gbh --out scene.c --prefix scene
./target/release/gbhua export scene.gbh --out tiles.gbtb
./target/release/gbhua export scene.gbh --out world.gbmb
./target/release/gbhua export scene.gbh --out tiles.2bpp
```

As dimensões devem ser múltiplos de 8 (8..2040). --size redimensiona por vizinho mais próximo. DMG usa quatro tons; CGB aproxima RGB555, quatro cores por tile e até oito paletas. A transparência é composta sobre branco. Tiles iguais são compartilhados; mais de 256 tiles únicos são rejeitados. Confira os avisos e a prévia.

Salve o projeto completo como .gbh. Projetos JSON antigos precisam apenas da extensão renomeada para .gbh; não há leitor da extensão anterior. GBTD/GBR/GBTB e GBMB/GBM são formatos de intercâmbio e não preservam todos os campos. GBMB também grava um .gbr com o mesmo nome: mantenha os dois juntos.

Para substituir arquivos use --force. Entrada e saída devem ter caminhos diferentes. Códigos: 0 sucesso, 1 projeto inválido, 2 erro de argumentos/E/S/conversão. Não há API de geração de imagens nem envio pela rede. As dependências mantêm suas licenças; veja LICENSE, THIRD_PARTY_NOTICES.md e DEPENDENCIES.json.

## GUI / Python

A GUI local e os bindings Python usam o mesmo núcleo. A GUI não está no pacote CLI independente. No ambiente local completo execute gbhua_gui; em Python use import gbhua. A GUI inclui menus agrupados, diálogos de salvamento, comparação DMG/CGB, sete tamanhos, entrada de 16 bytes, desfazer/refazer, zoom e rolagem do mapa.

