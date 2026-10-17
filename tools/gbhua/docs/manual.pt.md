# GBHUA

O Windows 10/11 x64 também exige [Microsoft Visual C++ v14 Redistributable x64](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist). Se faltar `VCRUNTIME140.dll`, instale o pacote x64 atual da Microsoft. Não baixe DLLs isoladas de outros sites.

## Download e início

Baixe o arquivo adequado em [Downloads da GUI](../GUI.md) e extraia todo o conteúdo. No Windows abra `GBHUA.exe`, mantendo a pasta `assets` ao lado. No Mac copie `GBHUA.app` para Aplicativos; escolha Apple Silicon ou Intel conforme o computador. A GUI não exige Rust nem Python para executar. O aplicativo não possui assinatura Developer ID nem notarização. Use o repositório oficial; se o macOS bloquear, confira a origem e autorize somente este aplicativo em Ajustes do Sistema > Privacidade e Segurança. Não desative a proteção do sistema.

A GUI usa rótulos em inglês; este documento é o manual traduzido. No Mac, `Ctrl` também indica Control. Os comandos de compilação começam na raiz do repositório. A CLI continua sem exigir tela ou GPU.

[English](manual.en.md) | [日本語](manual.ja.md) | [한국어](manual.ko.md) | [简体中文](manual.zh-CN.md) | [繁體中文](manual.zh-TW.md) | [Español](manual.es.md) | [Português](manual.pt.md) | [Français](manual.fr.md) | [Deutsch](manual.de.md)

O GBHUA converte PNG em tiles, paletas e mapas para Game Boy. Autor: DAISUKE OBA. Código original: MIT.

## Compilar e executar

```powershell
cd tools/gbhua
cargo run --release --locked --features gui --bin gbhua_gui
```

## GUI

Um documento novo sem alterações não mostra o aviso de alterações não salvas; edições reais continuam a exigir confirmação. No Windows, a GUI não abre o console por padrão. Para diagnóstico: `cargo run -p gbhua --bin gbhua_gui --features gui-console`.

Os títulos e os topos das telas estão alinhados. Passe o mouse sobre abrir, salvar e desfazer/refazer para ver os nomes. O botão de tamanho acima de Graphic muda as dimensões; ajustar, menos e mais acima de Map mudam o zoom. DMG/CGB fica no canto superior direito. Abaixo de 1180px de largura, alterne Tiles / Palettes na lateral esquerda. Use a roda ou a barra de rolagem para ver paletas e diagnósticos. Escape fecha o menu.

1. Selecione um tile à esquerda e um dos sete tamanhos em Graphic > 8x8. Gráficos grandes usam tiles consecutivos. Escolha a cor, arraste para pintar e clique com o botão direito para capturar uma cor.

2. TOOLS > FILL preenche uma região conectada; TILES > FILL TILE, o gráfico inteiro. FLIP X/Y e ROTATE transformam; EDIT > COPY/PASTE GRAPHIC duplica. UNDO/REDO mantém até 64 operações.

3. TOOLS > MAP posiciona o gráfico. Botão direito captura tile/paleta. RECT coloca 3x3; FILL preenche; ATTR aplica paleta e alterna inversão X; COLL/EVT alterna marcas. VIEW oferece Fit/1x/2x/4x, grade e marcas; use roda e barras para rolar.

4. Em BYTECODE, Ctrl+A e Ctrl+V colam 16 bytes. IMPORT altera o primeiro tile 8x8; EXPORT atualiza o texto. Setas/Home/End movem o cursor, Shift seleciona, Ctrl+C/X copia/recorta e Backspace/Delete apaga.

5. FILE > SAVE e EXPORT perguntam o destino. Com alterações (*), abrir/importar/fechar pergunta: Yes salva, No descarta, Cancel continua editando. Importar PNG substitui o projeto e permite desfazer.

6. SPRITES > NEW META cria um metasprite. ADD/DUP/CLEAR FRAME e PREV/NEXT FRAME gerenciam quadros. Em METASPRITE, clique adiciona partes e botão direito remove. DMG / CGB compara DMG/CGB sem alterar paletas salvas.

```text
0x78,0x78,0xCC,0xCC,0xCC,0xCC,0xC0,0xC0,
0xDC,0xDC,0xCC,0xCC,0x7C,0x7C,0x00,0x00
```

Salve o projeto completo como .gbh. Projetos JSON antigos precisam apenas da extensão renomeada para .gbh; não há leitor da extensão anterior. GBTD/GBR/GBTB e GBMB/GBM são formatos de intercâmbio e não preservam todos os campos. GBMB também grava um .gbr com o mesmo nome: mantenha os dois juntos.

## PNG / Python / CLI

As dimensões devem ser múltiplos de 8 (8..2040). --size redimensiona por vizinho mais próximo. DMG usa quatro tons; CGB aproxima RGB555, quatro cores por tile e até oito paletas. A transparência é composta sobre branco. Tiles iguais são compartilhados; mais de 256 tiles únicos são rejeitados. Confira os avisos e a prévia.

```powershell
python -m venv .venv
.\.venv\Scripts\python -m pip install maturin
.\.venv\Scripts\python -m maturin develop --features python
```

```python
import gbhua
project = gbhua.Project.from_image("artwork.png", mode="cgb")
project.save("scene.gbh")
project.save_preview("preview.png", "cgb", 2)
print(project.validate_json())
```

[CLI: Português](../README.pt.md)

## Importação PNG e solução de problemas

Use `FILE > IMPORT PNG` ou arraste o PNG. Selecione DMG/CGB antes. A importação substitui o projeto: salve primeiro. A seleção automática CGB considera o erro de cor para manter brilhos raros e contornos escuros, respeitando os limites do Game Boy. Para recuperar brilhos perdidos em uma importação antiga, importe o PNG original novamente; o `.gbh` reduzido não permite recuperar os pixels perdidos. `--size` é opção da CLI.

Selecione o tile, clique dentro de BYTECODE, digite exatamente 16 bytes e pressione IMPORT. EXPORT apenas atualiza o texto. Se o mapa mostrar cor ou orientação diferente, confira a paleta e os atributos de inversão. Use `.gbh` para manter todos os dados. Os erros aparecem na barra de status; cancelar o diálogo não salva.

No macOS/Linux execute `python3 -m venv .venv`, `source .venv/bin/activate`, `python -m pip install maturin` e `python -m maturin develop --features python`. A compilação dos bindings exige Python 3.10+ e Rust.
