# Bibliotecas da KITAQGB

[English](README.md) | [日本語](README.ja.md) | **Português (Brasil)**

[Abrir o manual da biblioteca KITAQGB em português](https://bartaro.github.io/kitaq-docs/pt/gb-library.html)

`wire3d_dmg` desenha gráficos 3D monocromáticos em wireframe (apenas as arestas) para Game Boy. Escolha `wire3d_dmg_96.c` para 128×96 ou `wire3d_dmg.c` para 128×120 e use as funções `Wire3DDMG_*`. Os arquivos `wire3d` e `dmg3d` permanecem como entradas de compatibilidade para esses respectivos perfis. Compile apenas uma entrada por ROM. `wire3d_cgb` mantém seu nome e continua sendo o renderizador separado para cores.

[Guia do renderizador](wire3d_dmg_guide.md) · [Guia em japonês](wire3d_dmg_guide_ja.md)

Esta pasta contém três tipos de arquivo:

- APIs públicas: bibliotecas reutilizáveis para incluir e compilar com o jogo.
- Unidades auxiliares: suporte opcional, declarações de registradores ou arquivos vazios que não constituem APIs públicas independentes.
- Documentação de referência: textos de consulta que não entram na compilação da ROM.

## Classificação

### APIs públicas

| Arquivos | Finalidade | Uso habitual |
| --- | --- | --- |
| `physics2d.h` / `physics2d.c` | Física 2D com caixas alinhadas aos eixos (AABB), integração da gravidade e resolução iterativa de contatos. | Inclua `physics2d.h` e compile `physics2d.c`. |
| `physics2d_circle.h` / `physics2d_circle.c` | Física de corpos circulares para jogos com bolas. | Inclua o cabeçalho e compile o fonte. |
| `physics3d.h` / `physics3d.c` | Física AABB em 3D com aceleração, rebote ponderado pela massa, indicadores de quebra e `kq3d_dot_q8_8()`. | Inclua o cabeçalho e compile o fonte. |
| `wire3d.h` / `wire3d.c` | Renderizador 3D em ponto fixo com buffer na WRAM, remoção de linhas ocultas e máscaras de oclusão da cena. | Inclua `wire3d.h` e compile `wire3d.c`. |
| `dmg3d.h` / `dmg3d.c` | Renderizador DMG com buffer de 128×120 em D000, desenho de linhas em assembly com ordem fixa e transferência D000→8900 condicionada ao STAT. | Inclua o cabeçalho e compile o fonte para desenhar em um buffer de 1bpp. |
| `wire3d_cgb.h` / `wire3d_cgb.c` | Renderizador colorido exclusivo de CGB a 8MHz: buffer 2bpp na WRAM, oclusão, recorte em assembly e apresentação sem rasgos via HBlank DMA. | Inclua o cabeçalho e compile o fonte em uma ROM exclusiva de CGB. |
| `system.h` / `system.c` | Inicialização, contador de quadros, espera por VBlank, callback cooperativo de VBlank e funções DI/EI. | Use em laços de jogo sincronizados por quadro. |
| `input.h` / `input.c` | Estado dos botões por quadro: mantido, recém-pressionado, recém-solto e repetição. | Use nos controles de menus, ação, quebra-cabeças e estratégia. |
| `vram.h` / `vram.c` | Fila de comandos de VRAM para tiles de BG, retângulos, blocos de mapa, memcpy e memset. | Enfileire durante o jogo e chame `vram_flush()` ou `vram_flush_now()` em um período seguro. |
| `sprite.h` / `sprite.c` | Cópia de trabalho da OAM, alocação, metasprites, animação, transferência DMA e verificação do limite por linha de varredura. | Inclua o cabeçalho e compile o fonte para desenhar com OBJ. |
| `fixed.h` / `fixed.c` | Ponto fixo Q8.8, `Vec2`, `KQRect`, clamp/min/max/lerp e testes básicos de retângulos. | Use em movimento, física, câmera e avaliações de IA. |
| `scene.h` / `scene.c` | Tabela leve de cenas e despacho de troca, atualização e desenho para título, jogo, pausa e estados semelhantes. | Use para organizar o fluxo de estados do jogo. |
| `entity.h` / `entity.c` | Pool em vetor fixo para até `ENTITY_MAX` entidades pequenas. | Callbacks recebem o ID da entidade e podem obter seus dados com `entity_get(id)`. |
| `danmaku.h` / `danmaku.c` | Pool de 96 projéteis em ponto fixo, leques de 32 direções, eventos de acerto e passagem próxima, composição de tiles de BG no CGB sem depender dos limites da OAM. | Inclua o cabeçalho e compile o fonte; consulte `danmaku_guide.md` e o jogo completo `ressen_gbc`. |
| `bank.h` / `bank.c` | Acesso a dados, ponteiros e chamadas distantes, além de troca básica de bancos MBC por operações intrínsecas. | Use como camada de apoio para dados distribuídos em bancos. |
| `asset.h` / `asset.c` | Tabela de descritores por ID de recurso e carregamento de dados brutos ou tiles. | Inclua o cabeçalho e compile o fonte; futuros geradores de `assets.h/c/json` podem usar esse formato. |
| `debug.h` / `debug.c` | Pequeno buffer na ROM para rastreamento, asserções e marcadores observáveis pelo KOKURA ou outro emulador. | Mantenha o processamento pesado dos registros fora da ROM. |
| `chain.h` / `chain.c` | Buffer circular de posições anteriores para cobras, cordas, trens e sprites articulados. | Use para fazer os segmentos seguirem o histórico de movimento. |
| `cgb_tile.h` | Declarações públicas das operações intrínsecas de tiles e atributos do CGB. | Inclua ao usar `__settile...` e funções relacionadas. |
| `cgb_palette.h` / `cgb_palette.c` | Camada de alto nível para paletas BG/OBJ do CGB. | Inclua o cabeçalho e compile o fonte. |
| `scroll.h` / `scroll.c` | Rolagem e tabelas de divisão da tela, baseadas em operações intrínsecas. | Inclua ao usar as funções `Scroll_*`. |
| `raster.h` / `raster.c` | Faixas de rolagem raster e perfis estruturados de deformação horizontal por linha. | Inclua `raster.h` e compile `raster.c` com `scroll.c`. |
| `camera.h` / `camera.c` | Câmera em ponto fixo 8.8 sobre `scroll.*`, funções globais simples e conversão entre mundo e tela. | Inclua o cabeçalho e compile o fonte. |
| `audio.h` / `audio.c` | Driver comum de música, efeitos, panorâmica, ondas e fades, com 68 índices de nota até o índice 67 (`G6`). | Inclua em projetos com áudio. |
| `audio_vblank.h` / `audio_vblank.c` | Driver de BGM por IRQ de VBlank com os mesmos 68 índices, fila de 16 registros na WRAM para músicas em bancos e gancho opcional por quadro. | Mantenha músicas acessadas por ponteiro direto no banco fixo 0 ou reabasteça a fila a partir de outros bancos. Ajuste o vetor `0x0040` com `scripts/patch_gb_vblank_irq.ps1`. |
| `link.h` / `link.c` | Transferência serial de bytes para cabo de comunicação e funções cooperativas de quatro participantes lógicos `Link4_*`. | Inclua em projetos com comunicação. |
| `link_packet.c` | Extensão de pacotes sobre `link.c`, com caixas de recepção por participante em `Link4_*`. | Compile com `link.c` somente se precisar enviar e ler pacotes. |
| `link_dmg07.h` / `link_dmg07.c` | Driver por consulta contínua e clock externo para o Nintendo DMG-07 Four Player Adapter físico. | Compile com `link_hwregs_gb.c`; é separado da API lógica `Link4_*`. |
| `rpg.h` | Declarações comuns de RPG, aventura e estratégia, além de operações intrínsecas de baixo nível. | Inclua ao usar essa família de funções. |
| `rng.c` | `rng8`, `rng16`, `rand_range`, `weighted_choice`, `rng_seed`, `rng_next8`, `rng_next16`, `rng_range` e `rng_chance`. | Compile ao usar as funções aleatórias de `rpg.h`. |
| `flags.c` | Conjunto de 2048 flags e armazenamento de estados de missões. | Compile ao usar flags e missões de `rpg.h`. |
| `rle.c` | Descompressão RLE simples no formato `[quantidade][valor]`, em RAM ou ROM distante. | Compile ao usar `rle_decode*` de `rpg.h`. |
| `text.c` | Janelas de texto por tiles, espera entre páginas, escolhas, texto em XY, números e funções alternativas de limpeza e janela. | Compile ao usar texto de `rpg.h`. |
| `menu.c` | Menus verticais, inventário mínimo e API de estado de menu que não bloqueia o laço principal. | Compile ao usar menus de `rpg.h`. |
| `script.c` | Pequeno interpretador de bytecode para sequências de RPG e aventura. | Compile ao usar scripts de `rpg.h`. |
| `map.c` | Carregador de mapas compactados com colisões, gatilhos, câmera e suporte opcional a metatiles 16×16. | Compile ao usar mapas de `rpg.h`. |
| `save.c` | Gravação, leitura, verificação e limpeza de SRAM no estilo MBC5, com cabeçalho, versão, comprimento e checksum. | Compile ao usar salvamento de `rpg.h`. |
| `slg_unit.c` | Alcance de movimento e ataque para jogos de estratégia. | Compile ao usar unidades táticas de `rpg.h`. |
| `slg_path.c` | Busca de caminhos em largura e preenchimento de alcance por custo de movimento. | Compile ao usar caminhos táticos de `rpg.h`. |
| `slg.h` / `slg_board.c` | Tabuleiro genérico, lista de jogadas e pilha de desfazer para jogos de tabuleiro ou sistemas táticos. | Inclua `slg.h` e compile `slg_board.c`; mantenha a avaliação específica do jogo fora da biblioteca. |

### Unidades auxiliares

| Arquivos | Finalidade | Observações |
| --- | --- | --- |
| `audio_hwregs_gb.c` | Declarações mínimas dos registradores da APU e da RAM de ondas. | Use apenas se outra unidade do projeto ainda não declarar esses registradores. |
| `link_hwregs_gb.c` | Declarações mínimas de `SB`, `SC`, `IF` e `IE` para comunicação. | Use apenas se os registradores seriais ainda não forem declarados em outro arquivo. |
| `cgb_tile.c` | Unidade de compilação intencionalmente vazia para as funções de tiles do CGB. | A API pública está em `cgb_tile.h`; compilar este fonte é inofensivo, mas não é necessário para executar as funções. |
| `math.c` | Tabela de senos em ROM, `MATH_SIN`. | Ainda não é documentada como uma API pública estável. Por enquanto, trate-a como unidade auxiliar de dados. |

### Documentação de referência

| Arquivos | Conteúdo |
| --- | --- |
| `README.md` | Esta visão geral e as instruções de compilação em inglês. |
| `wire3d_guide_ja.md` | Guia inicial em japonês para o renderizador 3D em wireframe. |
| `dmg3d_guide_ja.md` | Guia inicial em japonês para o renderizador DMG com buffer de trabalho. |
| `wire3d_cgb_guide.md` | Guia inicial do renderizador colorido exclusivo de CGB. |
| `physics_guide.html` | Guia de física em inglês. |
| `physics_guide_ja.html` | Guia de física em japonês. |

## Como compilar

Alguns comandos abaixo citam demos históricos de desenvolvimento que não estão neste repositório público, como `wire3d_cube_demo.c`. São modelos de compilação que exigem os fontes indicados. Para os programas de iniciação incluídos, use `../examples/build.ps1` e o manual HTML. O comando curto `kitaqgb` pressupõe que o executável esteja no PATH.

Compile o fonte do jogo junto com os fontes das bibliotecas necessárias:

```powershell
kitaqgb hwregs.c lib/audio.c main.c lib/physics2d.c lib/physics2d_circle.c lib/physics3d.c lib/cgb_palette.c lib/scroll.c lib/camera.c -I lib -o game.gb --profile=dev
```

Para gráficos 3D em wireframe, inclua o fonte do renderizador:

```powershell
.\kitaqgb.exe lib/wire3d.c examples/wire3d_minimal.c -I lib -o examples/wire3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

O exemplo incluído `examples/wire3d_minimal.c` inicializa todos os campos do modelo e gira um cubo na área de 128×96. Não precisa de imagens ou fontes externas.

Projetos DMG de 128×120 podem usar a entrada de compatibilidade de 120 linhas, `dmg3d.*`:

```powershell
.\kitaqgb.exe lib/dmg3d.c examples/dmg3d_minimal.c -I lib -o examples/dmg3d_minimal.gb --profile=dev --stack-bank=fixed --rst-disable --no-disasm
```

`DMG3D_Init()` configura uma superfície visível de 128×120, usando o buffer WRAM em D000 e tiles a partir de 0x8900. `DMG3D_BeginFrame()` reinicia apenas a oclusão; as transferências consomem e apagam os pixels. `DMG3D_EndFrame()` espera pelo VBlank e consulta o STAT durante a transferência, que pode continuar depois do VBlank. O exemplo incluído `examples/dmg3d_minimal.c` redesenha uma cruz a cada quadro com transferência das áreas alteradas. A transferência auxiliar é uma operação separada e compartilha o armazenamento de origem com o buffer principal.

Projetos coloridos exclusivos de CGB usam `wire3d_cgb.*`:

```powershell
kitaqgb lib/wire3d_cgb.c examples/wire3d_cgb_color_demo.c -I lib -o examples/wire3d_cgb_color_demo.gbc --profile=dev --stack-bank=fixed --rst-disable --cgb=cgb_only --rom-title=CGBWIRE3D
```

`Wire3DCGB_Init()` ativa a velocidade dupla do CGB, configura uma superfície BG de 128×96 em 2bpp e instala a paleta inicial de quatro cores. Tanto o par normal quanto o par `Fast` de funções de quadro usam a apresentação sem rasgos: os 3072 bytes em `0xD300–0xDEFF` são enviados por HBlank DMA ao banco de tiles de VRAM que não está sendo exibido, e a troca ocorre no VBlank, sem alterar o LCDC a cada quadro. O par `Fast` omite a verificação normal da fila de BG no fim do quadro. Use `Wire3DCGB_SetPaletteRGB15()`, `Wire3DCGB_SetLineColor()` e `Wire3DCGB_Draw*Color()` para definir as cores.

Para dados de nível de detalhe por direção gerados em CAD, `Wire3DCGB_DrawMaskedModel2D()` aceita deslocamentos de vértices com sinal já projetados e uma máscara compacta das arestas visíveis. A leitura das arestas e a rasterização em assembly ficam no banco 4 do renderizador; assim, desenhar um modelo não exige uma chamada entre bancos para cada linha.

Jogos com poucas alterações na tela e cópia de trabalho da OAM podem chamar `sprite_flush_oam()` e depois `Wire3DCGB_EndFrameSparseNow()` ao entrar no VBlank. Isso evita a espera inicial pelo próximo VBlank, mas o DMA e a apresentação ainda podem esperar, conforme a área alterada e a linha de varredura atual. Não há garantia de concluir tudo no mesmo VBlank.

Use as cores 1, 2 e 3 nas linhas do CGB. No modo normal de 128×96, as linhas combinam os bits de cor: sobrepor 1 e 2 produz 3. A cor 0 não apaga uma linha; limpe o quadro ou use as funções próprias de apagamento. `Wire3DCGB_DrawLine2D` e o desenho normal de modelos não registram os limites para transferência parcial. Para linhas que precisam desse registro, use `Wire3DCGB_DrawLineClipped2D`, ou chame `Wire3DCGB_InvalidateFrameHistory` para incluir toda a área visível na próxima transferência parcial.

O modo de 160×144 aloca no máximo 127 tiles por quadro. Uma falha de alocação ou coordenada fora dos limites no caminho rápido de linhas ativa `Wire3DCGB_GetFullScreenOverflow()` e impede novas escritas de pixels até reiniciar o quadro. Mantenha os vértices dentro da área escolhida. A margem da máscara triangular termina em X=127 no modo 128×96 e em X=159 na tela cheia. Siga as condições de mapeamento dos bancos de WRAM descritas na API, sobretudo para tela cheia e FastMap.

O [teste de regressão dos limites da máscara triangular CGB](../tests/library/wire3d_cgb_mask_bounds.c) é um programa completo que verifica os dois modos.

O demo histórico `examples/wire3d_cgb_hiddenline_demo.c` permite testar as linhas ocultas interativamente. `START` alterna a quantidade visível entre um e três objetos; `B` escolhe o objeto; o direcional move em X/Y; `A`+cima/baixo move em Z; `A`+esquerda/direita gira em Z; e `SELECT`+direcional gira em X/Y em passos de 22,5 graus. A remoção de linhas ocultas e a oclusão entre objetos permanecem ativas, e corpos que colidem se afastam. Esse demo de desenvolvimento não faz parte do repositório público.

Exemplos de compilação para RPG, aventura e estratégia:

```powershell
kitaqgb examples/example_rpg_text.c lib/text.c lib/menu.c -I lib -o text.gb --profile=dev
kitaqgb examples/example_adv_script.c lib/text.c lib/flags.c lib/script.c -I lib -o script.gb --profile=dev
kitaqgb examples/example_slg_cursor.c lib/map.c lib/slg_unit.c lib/slg_path.c -I lib -o slg.gb --profile=dev
```

Compilação de verificação básica do runtime padrão:

```powershell
kitaqgb lib/text.c lib/menu.c lib/map.c lib/scroll.c lib/camera.c lib/rng.c lib/save.c lib/system.c lib/input.c lib/vram.c lib/sprite.c lib/fixed.c lib/scene.c lib/entity.c lib/bank.c lib/asset.c lib/debug.c lib/chain.c lib/physics2d.c lib/slg_board.c examples/standard_library_smoke.c -I lib -o examples/standard_library_smoke.gb --profile=dev --rom-title=STDLIBSMK --no-disasm
```

Em projetos com comunicação serial, compile a unidade de registradores antes do núcleo de comunicação:

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c main.c -I lib -o game.gb --profile=dev
```

A comunicação lógica cooperativa de quatro participantes usa os mesmos arquivos. O anfitrião chama `Link4_InitHost(slot_count)` e escolhe o participante com `Link4_SelectPeer()` ou `Link4_SendPacketTo()`. Os demais chamam `Link4_InitPeer(local_slot, slot_count)` e se comunicam com o anfitrião no slot 0.

Os exemplos dos participantes também podem ser compilados como versões prontas para cada slot:

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot1.c -I lib -o peer1.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot2.c -I lib -o peer2.gb --profile=dev
kitaqgb lib/link_hwregs_gb.c lib/link.c lib/link_packet.c examples/link4_demo_peer_slot3.c -I lib -o peer3.gb --profile=dev
```

Para o DMG-07 físico, use o driver dedicado de consulta contínua:

```powershell
kitaqgb lib/link_hwregs_gb.c lib/link_dmg07.c main.c -I lib -o dmg07.gb --profile=dev
```

Chame `LinkDmg07_Poll()` continuamente: os bytes do adaptador chegam em intervalos muito menores que um quadro de vídeo. Chame `LinkDmg07_TickFrame()` uma vez por VBlank para atualizar os contadores saturantes de ausência de comunicação e de espera pelo handshake. O driver sempre configura o clock externo com `SC=$80`, responde aos pings com `88 88 RATE 01` e permite apenas ao jogador físico 1 solicitar a transmissão com `AA AA AA AA`.

Depois que todos os consoles recebem `CC CC CC CC`, cada pacote de quatro bytes transmitido a todos contém um byte por slot físico. O adaptador repassa os dados no pacote seguinte ao envio; por isso, o driver descarta o primeiro pacote indefinido e disponibiliza números de sequência de envio e recepção. `LinkDmg07_RequestRestart()` espera pelo próximo limite de pacote, envia `FF FF FF FF` alinhado e para assim que recebe o indicador completo de quatro bytes FF do adaptador. Um timeout de silêncio durante a transferência agenda esse reinício automaticamente, preservando a posição atual no pacote. Quando os clocks voltam, o pacote em andamento pode terminar antes da recuperação.

Inclua os cabeçalhos necessários no código do jogo:

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

## Observações de uso

Os oito últimos índices de nota atualmente reutilizam as frequências da oitava anterior. Ter 68 índices não significa ter 68 alturas distintas.

- `inv_mass_q8 == 0` representa um corpo estático.
- `Wire3D_Init()` usa uma superfície BG de 128×96, buffer WRAM a partir de `0xD000` e tiles a partir de `0x8900`. Compile com `--stack-bank=fixed`.
- `Wire3D_BeginFrame()` limpa o buffer WRAM; `Wire3D_EndFrame()` espera pelo VBlank e copia em blocos para a VRAM, respeitando o STAT.
- Os ângulos de Wire3D têm 16 passos. O caminho básico de modelos aceita até `WIRE3D_MODEL_VERTEX_LIMIT` vértices por modelo.
- Use `Wire3D_DrawScene()` quando objetos com faces representadas por linhas se sobrepuserem. A função desenha os mais próximos primeiro e acumula suas máscaras de faces, ocultando as linhas distantes de forma conservadora.
- `Wire3DCGB_Init()` é exclusivo de CGB e faz a troca de velocidade por KEY1/STOP. Compile com `--cgb=cgb_only`; não combine `wire3d_cgb.*` com o renderizador DMG `wire3d.*` na mesma ROM.
- Os pares normal e `Fast` mantêm o quadro anterior visível até terminar a transferência para o banco de tiles inativo. Prefira `Fast` quando a cena não usar a fila de escrita de tiles de BG.
- Compile o arquivo que declara `NR10..NR52` e `WAVE0..WAVE15` antes de `lib/audio.c`.
- `lib/audio_hwregs_gb.c` já fornece essas declarações. Não o compile junto de outro arquivo que declare os mesmos registradores de áudio.
- `cgb_tile.h` expõe diretamente operações intrínsecas. `lib/cgb_tile.c` é apenas uma unidade vazia e pode ser omitida nas compilações normais.
- A API pública de `cgb_palette.h` usa os nomes `cgb_*`.
- Chame `Audio_SetMusicEnabled()` e `Audio_SetSfxEnabled()` quando o menu ou as configurações alterarem a ativação do áudio.
- `Audio_PlaySFX()` registra o banco ROM visível no momento da chamada. Use `Audio_PlaySFXBanked(bank, sfx, priority)` quando o banco dos dados do efeito for conhecido explicitamente.
- `AUDIO_CMD_NOTE` e `AUDIO_CMD_SET_INST` mantêm a codificação antiga dos canais: `0=CH1`, `1=CH2`, `2=CH4`, `3=CH3`.
- Para uma onda própria do CH3, passe a `Audio_LoadCustomWave()` 16 bytes com 32 amostras de 4 bits compactadas.
- O fade de `Audio_FadeToMasterVolume()` avança em `Audio_Update()`. Continue chamando essa atualização a cada quadro durante o fade.
- `audio_vblank.c` define o símbolo de vetor de IRQ de VBlank `__kq_vblank_vector`. Cada evento de BGM ocupa cinco bytes: `delay, ch2_note, ch1_note, ch3_note, ch4_noise_param`. Use `AUDIO_VBLANK_REST`, `AUDIO_VBLANK_LOOP` e `AUDIO_VBLANK_END` para pausa, repetição e fim.
- Músicas de VBlank acessadas por ponteiro direto devem ficar no banco fixo. O modo de fila pode ser abastecido a partir de outros bancos. Depois do link com `lib/audio_vblank.c`, execute `scripts/patch_gb_vblank_irq.ps1 <rom> <map>` para fazer o vetor `0x0040` saltar para a ISR e atualizar os checksums da ROM.
- Não combine `audio_vblank.c` com outra biblioteca ou rotina do jogo que também controle o vetor VBlank `0x0040`, salvo se criar um despachante de IRQ compartilhado.
- `Scroll_SplitCommit()` ativa automaticamente os bits IE `0x01 | 0x02` e usa os tratadores VBlank/STAT do compilador para executar a divisão da tela.
- As funções de divisão reservam os vetores `0x0040` e `0x0048` nessa compilação. Por enquanto, não as combine com rotinas VBlank/STAT próprias.
- Compile as declarações de `SB`, `SC`, `IF` e `IE` antes de `lib/link.c` / `lib/link_packet.c` ou de `lib/link_dmg07.c`.
- `lib/link_hwregs_gb.c` fornece essas declarações. Não o compile junto de outro arquivo que declare os mesmos registradores seriais.
- A biblioteca de comunicação não define o vetor serial `0x0058`. Se ativar o modo de interrupção, chame `Link_OnSerialIRQ()` a partir de sua rotina de IRQ ou despachante.
- A camada de pacotes guarda apenas um pacote por posição de recepção. Processe-a regularmente em um laço sincronizado por quadro.
- `Link4_*` modela quatro participantes cooperativos, com seleção pelo anfitrião. Apenas um participante está ativo no fio por vez; o anfitrião precisa alternar entre eles.
- `Link4_TryReadByteFrom()` e `Link4_HasPacketFrom()` expõem caixas de recepção por participante, permitindo consultar vários sem perder a identificação da origem.
- `Link_ReadPacket()` continua oferecendo a visão antiga do último pacote. Em fluxos de quatro participantes, use `Link4_ReadPacketFrom()`.
- `Link4_*` não implementa o comportamento elétrico ou o protocolo do Nintendo DMG-07. Para o acessório físico, use `link_dmg07.c` e não o compile com `link.c` na mesma ROM.
- No DMG-07, `GetConnectedMask()` usa os bits 0–3 para os jogadores físicos 1–4. Durante a transmissão, conserva o resultado do último ping. A composição dos participantes só pode ser atualizada na fase de ping.
- Um reinício pendente do DMG-07 não avança sem clocks do adaptador. O tráfego de recuperação é descartado, sem ser entregue como dados ao programa. Se o acessório foi desligado e voltou em outra fase, em vez de apenas interromper o clock, reinicialize explicitamente o driver e a sessão.
- As bibliotecas de física tratam apenas posição e velocidade lineares, sem dinâmica angular.
- Mantenha poucos corpos ativos em hardware da classe Game Boy; por exemplo, de 8 a 24.
- Ajuste a gravidade, a velocidade máxima e o número de iterações de resolução de contatos para cada mundo conforme as necessidades do jogo.
- Para jogos semelhantes a bilhar, prefira `physics2d_circle.*` à biblioteca AABB.
- Na organização atual, não crie bibliotecas separadas chamadas `random`, `collision`, `ui`, `tilemap`, `dialog`, `board_game` ou `simple_physics`. Use, respectivamente, `rng`, `physics2d`, `text`/`menu`, `map`, `script`, `slg` e `physics2d`.
- `scene.c` e `entity.c` evitam argumentos do tamanho de ponteiros nas chamadas indiretas. O caminho atual de chamadas por ponteiro de função da KITAQGB é mais confiável sem argumentos ou com IDs de um byte.
