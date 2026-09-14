# KITAQGB

[English](README.md#english) | [日本語](README.md#japanese) | **Português (Brasil)**

**[Manual do compilador em português](https://bartaro.github.io/kitaq-docs/pt/kitaqgb.html)** · **[Manual da biblioteca em português](https://bartaro.github.io/kitaq-docs/pt/gb-library.html)**

## Origem do nome

A KITAQGB começou como um fork do NORCAL, o compilador C para NES associado à Zachtronics. O aviso de direitos autorais de Keith Holman, autor original do NORCAL, é mantido.

O nome NORCAL vem do norte da Califórnia (Northern California). Inspirado nessa referência geográfica, o autor escolheu o nome KITAQGB com base em Kitakyushu, a cidade onde nasceu e cresceu. KITAQ + GB combina Game Boy com **北九 (キタキュー, Kitakyū)**, o apelido de Kitakyushu, na província de Fukuoka, no Japão. KITAQ se pronuncia como o japonês “キタキュー”; a orientação de pronúncia em inglês é **kee-tah-KYOO**, com transcrição fonética **/ˌkiːtɑːˈkjuː/**. O Q final tem o som do nome da letra Q em inglês. KITAQGB se lê **kee-tah-KYOO jee bee**, pronunciando G e B separadamente, também em inglês.

O nome KITAQGB tem dois sentidos. **Kernel-Informed Toolchain for AI-Quality Game Boy Development** expressa o objetivo de um conjunto de ferramentas que entende a máquina de destino e apoia tanto quem programa quanto a IA generativa.

O outro sentido é **Kids' Imagination Transformed into Actual Quests in Game Boy Forests**: uma ferramenta que transforma a imaginação das crianças em aventuras de verdade nas florestas do Game Boy. Ele expressa o desejo de transformar pequenas ideias, rabiscos e protótipos criados com ajuda de IA em aventuras que as pessoas possam realmente jogar.

## Estado do projeto: prévia pública

KITAQGB e KOKURA estão disponíveis como ferramentas de desenvolvimento em versão de prévia pública.

Podem ser usados em experimentos, exemplos, criação de jogos com auxílio de IA, pesquisa de compiladores, depuração em emulador e validação de fluxos de trabalho. Os projetos continuam em desenvolvimento ativo: APIs, opções de linha de comando, formatos de saída, diagnósticos e comportamento podem mudar entre versões.

Essas versões podem conter erros, recursos incompletos e mudanças incompatíveis. Verifique o código gerado, o comportamento do emulador, os diagnósticos de temporização e os relatórios antes de usá-los em produção ou em uma publicação.

**Kernel-Informed Toolchain for AI-Quality Game Boy Development**

A KITAQGB é um conjunto de ferramentas C de código aberto para desenvolver software independente, ou homebrew, para Game Boy e Game Boy Color. Foi concebida para trabalhar bem com diagnósticos e desenvolvimento assistido por IA: escreva pequenos programas C, compile-os em ROMs `.gb` ou `.gbc` e use as observações do emulador para melhorar o jogo.

A KITAQGB não tem vínculo com a Nintendo e não é endossada, patrocinada nem aprovada por ela. Game Boy e Game Boy Color são marcas da Nintendo.

## O que a KITAQGB oferece

A KITAQGB reúne um compilador C e bibliotecas de apoio para homebrew da família Game Boy. Amplia a base do NORCAL com um fluxo de desenvolvimento voltado à criação de jogos com ferramentas atuais e auxílio de IA.

O projeto se concentra em:

- compilar C em ROMs para Game Boy;
- desenvolver homebrew para Game Boy e Game Boy Color;
- produzir diagnósticos úteis para IA e relatórios de compilação reproduzíveis;
- gerar ROMs, cabeçalhos e código de baixo nível para a família LR35902;
- oferecer bibliotecas de paletas, tiles, rolagem, câmera, áudio, comunicação e recursos de RPG, aventura e estratégia;
- trabalhar com o KOKURA CLI em testes, rastreamento e depuração no emulador.

A KITAQGB **não inclui ROMs comerciais, BIOS da Nintendo, arquivos do SDK da Nintendo, recursos proprietários nem materiais oficiais de desenvolvimento da Nintendo**.

## Plataformas de destino

A KITAQGB gera ROMs homebrew compatíveis com Game Boy e Game Boy Color. Também permite criar jogos exclusivos de Game Boy Color, quando o projeto usa intencionalmente os recursos do CGB. As extensões de saída usuais são:

```text
*.gb
*.gbc
```

Teste as ROMs em um emulador e, quando viável, em hardware real ou com um flashcart adequado. Temporização, interrupções, acesso à VRAM/OAM, áudio, cabo de comunicação e troca de bancos exigem atenção às particularidades do hardware.

## Organização do repositório

Os fontes do compilador ficam na subpasta `kitaqgb/`. O executável Release pronto e sua configuração ficam na raiz; bibliotecas e exemplos têm pastas próprias.

```text
kitaqgb/                  # Repository root
├─ kitaqgb/               # Compiler build sources
│  ├─ *.cs
│  ├─ app.config
│  └─ kitaqgb.csproj
├─ kitaqgb.exe            # Prebuilt Release compiler
├─ kitaqgb.exe.config     # .NET Framework runtime configuration
├─ lib/                # C support libraries
├─ examples/           # Tutorial programs and original font
├─ scripts/build.ps1   # Rebuild the Release executable
├─ LICENSE
└─ LICENSE.ja
```

O compilador fornecido exige Windows com .NET Framework 4.8. Baixe o ZIP do repositório para manter juntos o executável, a configuração de execução, as bibliotecas e os avisos de licença. Para recompilar, instale também o .NET Framework 4.8 Developer Pack e o Visual Studio Build Tools. A partir da raiz do repositório:

```powershell
.\scripts\build.ps1
.\kitaqgb.exe --help
.\examples\build.ps1
```

A compilação Release copia o executável e a configuração para a raiz. As compilações Debug permanecem em `kitaqgb/bin/Debug` e não substituem o compilador Release distribuído. Caches de compilação e arquivos PDB não são distribuídos. O [registro da compilação binária](BINARY_BUILD.json) contém os dados de entrada e os hashes SHA-256.

## Requisitos

O ambiente principal de compilação é Windows com os arquivos de referência do .NET Framework 4.8 e Visual Studio ou Visual Studio Build Tools com MSBuild. O arquivo de projeto usa o formato clássico de C#, com destino `.NET Framework v4.8`.

Outros sistemas podem funcionar com Mono/MSBuild, dependendo dos assemblies de referência instalados. O caminho de compilação com suporte principal é Windows com MSBuild.

## Compilar a KITAQGB

Na raiz do repositório, execute:

```powershell
msbuild kitaqgb\kitaqgb.csproj /p:Configuration=Release
```

Depois de uma compilação bem-sucedida, o projeto copia o executável para a raiz:

```text
kitaqgb.exe
```

Confira a ajuda da linha de comando:

```powershell
.\kitaqgb.exe --help
```

## Primeiros passos

Crie um pequeno projeto a partir do modelo:

```powershell
.\kitaqgb.exe template hello.c --overwrite
```

Compile o programa:

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=dev --fast-build --cache
```

Para uma compilação com perfil de distribuição:

```powershell
.\kitaqgb.exe hello.c -o hello.gb --profile=release --cache
```

Abra a ROM no emulador de Game Boy/Game Boy Color de sua preferência, ou no KOKURA CLI para usar suas funções de depuração e observação.

## Usar as bibliotecas incluídas

A pasta `lib/` contém código C reutilizável. Compile os fontes necessários junto com seu jogo e indique `-I lib` para localizar os cabeçalhos.

Exemplo com áudio, paletas, rolagem, câmera e física. Os comandos de várias linhas abaixo usam o caractere de continuação do **cmd.exe**:

```cmd
.\kitaqgb.exe main.c ^
  lib\audio_hwregs_gb.c lib\audio.c ^
  lib\cgb_palette.c lib\scroll.c lib\camera.c ^
  lib\physics2d.c lib\physics2d_circle.c lib\physics3d.c ^
  -I lib -o game.gb --profile=dev --fast-build --cache
```

Exemplo de comunicação serial:

```cmd
.\kitaqgb.exe lib\link_hwregs_gb.c lib\link.c lib\link_packet.c main.c ^
  -I lib -o link_game.gb --profile=dev --fast-build --cache
```

Exemplo com funções de RPG, aventura e estratégia:

```cmd
.\kitaqgb.exe main.c lib\text.c lib\menu.c lib\flags.c lib\script.c lib\map.c lib\save.c ^
  -I lib -o rpg.gb --profile=dev --fast-build --cache
```

Consulte [lib/README.pt-BR.md](lib/README.pt-BR.md) para ver a classificação e as observações de uso das bibliotecas.

## Opções frequentes de linha de comando

```text
-o <file>                  Caminho da ROM de saída
-I <dir>                   Pasta de cabeçalhos
--profile=dev              Perfil de desenvolvimento
--profile=release          Perfil de distribuição
--fast-build / --fast      Compilação rápida para desenvolvimento
--cache                    Ativar o cache de compilação
--no-cache                 Desativar o cache de compilação
--disasm                   Gerar a listagem do código desassemblado
--no-disasm                Não gerar a listagem desassemblada
--diag-json <file>         Gravar os diagnósticos em JSON
--machine-readable         Preferir saída para processamento por ferramentas
--deps-out <file>          Gerar informações de dependências
--debug-output <dir>       Pasta para saídas de depuração e apoio
--strict                   Tratar determinados avisos como erros
--permissive               Relaxar determinados diagnósticos
--stack-bank=fixed|wramx1  Escolher o modelo de banco da pilha
--stack-top=<addr>         Escolher o endereço do topo da pilha
--stack-reserve=<bytes>    Reservar espaço para a pilha
```

Para consultar as opções exatas aceitas pela sua versão:

```powershell
.\kitaqgb.exe --help
```

## Trabalhar com o KOKURA CLI

O KOKURA CLI complementa a KITAQGB como emulador e depurador. Um fluxo típico é:

1. Escrever ou gerar o código C do jogo.
2. Compilar com a KITAQGB.
3. Executar a ROM no KOKURA CLI.
4. Registrar diagnósticos, rastreamentos, símbolos, medições de tempo e relatórios do emulador.
5. Usar os resultados na próxima alteração do código ou etapa de depuração.

No desenvolvimento assistido por IA, isso permite transformar erros de compilação e observações de execução em tarefas de correção específicas.

## Princípios de desenvolvimento

A KITAQGB foi feita para uma plataforma de jogos de 8 bits com pouca memória, bancos e exigências de temporização. Não pretende ser um compilador C moderno de uso geral.

O projeto prioriza código gerado previsível, diagnósticos claros, pequenos exemplos reproduzíveis e relatórios úteis tanto para pessoas quanto para ferramentas de IA. Oferece controle de baixo nível quando necessário e bibliotecas de apoio acessíveis quando possível. A intenção é facilitar o desenvolvimento para Game Boy sem ocultar completamente a máquina.

## Marcas e independência

A KITAQGB é um projeto independente de código aberto para desenvolvimento homebrew. Não tem vínculo com a Nintendo e não é endossada, patrocinada nem aprovada por ela. Game Boy e Game Boy Color são marcas da Nintendo.

Não acrescente a este repositório logotipos da Nintendo, arte de embalagens oficiais, fontes oficiais, BIOS, dados de ROMs comerciais ou recursos proprietários de jogos sem ter os direitos necessários.

## Licença

A KITAQGB é distribuída sob a licença MIT. Ela deriva do NORCAL, cujo aviso original é:

```text
Copyright 2019 Keith Holman
```

As modificações e adições da KITAQGB têm o seguinte aviso:

```text
Copyright (c) 2026 DAISUKE OBA
```

O aviso original de direitos autorais do NORCAL e o texto da licença MIT devem ser preservados nas cópias ou em partes substanciais do software. Consulte [LICENSE](LICENSE) e [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Contribuir

Antes de enviar alterações:

- Não inclua ROMs protegidas, BIOS, recursos extraídos de jogos comerciais nem materiais de SDKs oficiais.
- Mantenha saídas geradas como `bin/`, `obj/`, `target/`, `dist/`, `*.exe`, `*.dll` e `*.pdb` fora dos commits de código, salvo quando houver um motivo específico de distribuição.
- Prefira casos pequenos e reproduzíveis para erros do compilador ou da geração de código.
- Ao adicionar bibliotecas, documente o comando de compilação e as declarações de registradores necessárias.
- Escreva diagnósticos claros, que permitam a uma pessoa ou a um agente de programação com IA identificar a próxima ação.

## Estado desta edição

Este repositório foi preparado para uma primeira publicação da KITAQGB. Interfaces, bibliotecas, diagnósticos e integração com outras ferramentas podem evoluir à medida que o projeto avança.

## Compilação e primeiro uso

Use Windows, .NET Framework 4.8 Developer Pack e Visual Studio Build Tools com MSBuild. Execute em um prompt Developer PowerShell:

```powershell
MSBuild.exe .\kitaqgb\kitaqgb.csproj /t:Build /p:Configuration=Release
.\kitaqgb.exe --help
.\examples\build.ps1
```

## Manuais e licenças

- [Manual da KITAQGB em português](https://bartaro.github.io/kitaq-docs/pt/kitaqgb.html) · [Biblioteca em português](https://bartaro.github.io/kitaq-docs/pt/gb-library.html)
- [Fontes do manual para leitura offline](https://github.com/bartaro/kitaq-docs)
- [Licença](LICENSE) · [Tradução de referência em japonês](LICENSE.ja)
