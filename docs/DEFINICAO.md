# EFFECTOR: Definição do Projeto

> Documento vivo. Registra tudo o que foi decidido para o desenvolvimento.
> Última atualização: 24/11/2026 · Status: **v0.1.0 implementada** (todas as etapas 1–5 entregues)

---

## 1. Visão

App nativo para macOS, **100% pessoal**, que aplica efeitos visuais em imagens usando um **sistema de layers**. Junta:

- a **profundidade técnica do Dither Boy**: dithering, paletas, halftone;
- a **amplitude estética e a fluidez do effect.app**: filme, VHS, glow, gradient map, presets;
- **performance nativa**: Metal, offline, sem conta e sem limite artificial de export.

**Fora do escopo (por enquanto):** vídeo, animação e keyframes, máscaras, várias imagens abertas ao mesmo tempo, lote, export SVG.

---

## 2. Plataforma e ambiente

| Item | Decisão |
|---|---|
| Sistema | Apenas macOS recente (macOS 26+). Máquina de uso: MacBook Pro M1 Pro, 16 GB |
| Stack | Swift + SwiftUI (AppKit onde necessário) + Metal / Core Image |
| Instalação | App compilado e instalado em `/Applications/EFFECTOR.app`. O usuário abre como um app normal |
| Código | 100% mantido pelo Claude. O usuário não lê nem edita código |
| Versionamento | Git + repositório **privado** no GitHub (`effector`). Commit e push automáticos a cada entrega |
| Nome | **EFFECTOR** (provisório) |

---

## 3. Arquivos: entrada, projeto e export

**Entrada**
- JPG, PNG, HEIC, TIFF e **RAW** (CR3, ARW, NEF, DNG etc., via decodificador nativo do macOS).
- Tamanho típico de até cerca de 6000×4000.
- Formas de abrir: ⌘O, arrastar para a janela, **colar da área de transferência (⌘V)**.
- Uma imagem por vez. A arquitetura deixa caminho aberto para várias no futuro.

**Projeto**
- Formato próprio **`.effector`**: pacote com a imagem original, a pilha de layers (JSON) e uma miniatura.
- Reabrir mantém tudo editável.

**Export**
- **JPG** (qualidade ajustável) e **PNG** (com transparência, quando houver).
- Escala **1x ou 2x** do original (máximo 2x).
- **O export fica igual ao preview, só mais nítido.** Todos os parâmetros de tamanho (pixel do dither, ponto do halftone, grain…) são **relativos**: medidos em pixels de uma imagem de referência de 1000 px no lado maior. Assim um preset fica igual em fotos de qualquer tamanho, e no preview, no export 1x e no 2x.
- Testado: export 2x de 12032×12032 px com 4 efeitos em cerca de 10 s, usando 1,3 GB de memória.
- **Copiar o resultado** para a área de transferência (⌘⇧C).
- O export roda em segundo plano, com barra de progresso.

---

## 4. Sistema de layers

- **Ordem:** cada layer processa o resultado de tudo o que está abaixo dele. O topo da lista é aplicado por último.
- **Por layer:** ligar/desligar, solo, opacidade, blend mode, renomear, duplicar, excluir, resetar parâmetros.
- **Blend modes:** Normal, Multiply, Screen, Overlay, Soft Light, Hard Light, Darken, Lighten, Difference, Color, Luminosity.
- **Grupos:** composite isolado. O grupo recebe a imagem de baixo, processa os filhos e devolve o resultado com opacidade e blend **do grupo**. Grupos podem ficar dentro de grupos e podem ser recolhidos.
- **Interações:** arrastar e soltar para reordenar e mover para dentro ou fora de grupos, ⌘G para agrupar, ⇧⌘G para desagrupar.
- **Crop:** proporções prontas (Livre, Original, 1:1, 4:5, 3:2, 2:3, 16:9, 9:16) aplicadas **antes** de todos os efeitos.
- **Máscaras:** fora do MVP (fase futura).

---

## 5. Presets

- Um preset é uma **pilha de layers salva**, com nome e miniatura.
- **Aplicar um preset adiciona a pilha por cima, como um grupo** (não substitui o que já existe).
- **Importar e exportar** presets como arquivo (`.effectorpreset`).
- **Presets de fábrica** (removíveis no futuro), de 10 a 15 looks para testar todos os efeitos. Exemplos: Portra Warm, Tri-X Push, Cinestill Halation, VHS 1994, Thermal Cam, Gameboy, Riso Duo, CMYK Newsprint, ASCII Terminal, Dreamy Bloom, Surveillance (blob tracker), Ghost Trail.
- Paletas e gradientes também têm presets próprios, importáveis e exportáveis.

---

## 6. Efeitos

> Regra global: **todo efeito ou modo preto e branco é neutro de verdade** (R = G = B exatamente). A luminância é calculada em espaço linear e os canais são colapsados no final. Nunca pode puxar para nenhuma cor.

### 6.1 Ajustes básicos
- **Exposure**, **Contrast**, **Highlights/Shadows**, **Whites/Blacks**
- **Curves** (RGB + canais R, G e B, com pontos arrastáveis)
- **HSL** (por faixa de cor: hue, saturação, luminância)
- **Hue**, **Saturation**, **Vibrance**, **Temperature/Tint**
- **Black & White** (neutro, com mixer por canal)

### 6.2 Dithering
- Algoritmos clássicos:
  - **Ordered:** Bayer 2×2, 4×4, 8×8
  - **Difusão de erro:** Floyd-Steinberg, Atkinson, Jarvis-Judice-Ninke, Stucki, Sierra
  - **Threshold** e **Random**
- Controles: **escala do pixel**, contraste e brilho antes do dither, intensidade da difusão, serpentina, quantidade de cores.
- **Modo:** monocromático ou **paleta de cores**.
- **Paletas:**
  - **clássicas** prontas (Game Boy, CGA, EGA, PICO-8, C64, Mac Classic, 1-bit…);
  - **extraídas da imagem** (k-means, com N cores);
  - **customizadas** (editor de cores);
  - **importar e exportar** (.hex / .gpl / .txt / .json). Formato .ase ficou para o backlog.

### 6.3 Grain
- Mono ou colorido, tamanho, intensidade, aspereza, resposta por tons (mais nas sombras ou nos médios), seed.

### 6.4 Gradient Map
- **Editor de gradiente** com pontos arrastáveis.
- **Presets:** thermal (Iron, Rainbow, White-hot, Black-hot, Arctic), duotones e outros.
- Salvar, importar e exportar gradientes.

### 6.5 Halftone
- Modos: **P&B** e **CMYK** (ângulos por canal, com padrões clássicos 15°/75°/0°/45°).
- **Escala/frequência**, **ângulo**, **formas do ponto:** círculo, quadrado, losango, linha, elipse, cruz.
- Cor de fundo e de tinta, suavização, ganho de ponto.

### 6.6 Glow / Bloom / Halation
- **Bloom:** limiar, raio, intensidade, tingimento.
- **Glow:** brilho suave geral.
- **Halation:** brilho avermelhado nas altas luzes, típico do filme.

### 6.7 Film Emulation
- Estoques famosos **coloridos**: Portra 160/400, Ektar 100, Gold 200, Fuji Pro 400H, Superia, Velvia 50, Cinestill 800T.
- Estoques famosos **P&B (neutros de verdade)**: Tri-X 400, HP5, T-Max, Delta 3200, Acros.
- Cada estoque é uma combinação de curva, cor, grain e halation, com **intensidade** e ajustes finos.
- **Suporte a LUTs `.cube`** como layer próprio.

### 6.8 Trail / Ghosting ("motion")
- Cópias deslocadas e esmaecidas da imagem: direção, distância, número de cópias, decaimento, blend.
- Variações: direcional, radial (zoom) e echo com deslocamento de cor.

### 6.9 VHS
Cada sub-efeito tem seu próprio toggle:
- tracking / jitter de linhas;
- chroma bleed / deslocamento de cor;
- ruído de fita;
- scanlines;
- tape wobble / onda;
- perda de saturação e de nitidez;
- dropout (falhas brancas);
- **OSD** ("PLAY ▶", timestamp editável).

### 6.10 ASCII
- **Presets de charset** (clássico, blocos, densidade, binário, braille…).
- Tamanho da célula, contraste, inverter.
- Monocromático ou com **paleta de cores**.
- É só efeito visual, sem export em texto.

### 6.11 Blob Tracker
- Visual estilo TouchDesigner / vigilância: **caixas, marcações, linhas conectando os blobs e rótulos**.
- **Detecção** (escolhida após pesquisa, no modo "Auto" com alternativas):
  - **Brilho:** limiar de luminância e componentes conectados (padrão, rápido e previsível);
  - **Cor:** distância de uma cor-alvo;
  - **Bordas/contornos:** Vision (detecção de contornos do macOS);
  - **Saliência:** Vision (áreas de interesse).
- Controles: número máximo de blobs, tamanho mínimo e máximo, sensibilidade.
- **Estilo:**
  - forma da marcação (caixa, cantos, cruz, círculo);
  - cor, espessura e fonte;
  - tipo de rótulo (ID, coordenadas, tamanho, texto custom);
  - linhas de conexão (nenhuma, sequencial, mais próximo);
  - preenchimento ou inversão dentro da caixa.

---

## 7. Interface e experiência

- **Layout:** layers à esquerda, canvas ao centro e inspector do layer selecionado à direita, com toolbar no topo. A vibe mistura Photoshop/Figma com macOS nativo.
- **Visual:**
  - base em cinzas neutros;
  - **amarelo `#FFBB00` só como destaque** (seleção, ativo, foco), nunca como base;
  - materiais nativos do macOS.
- **Canvas:** zoom e pan suaves (pinça, scroll, ⌘+ e ⌘−, ⌘0 para ajustar à janela, ⌘1 para 100%).
- **Preview progressivo:** resposta instantânea em resolução reduzida, refinada logo em seguida.
- **Antes/Depois:** botão na toolbar e atalho de teclado (toggle).
- **Navegador de efeitos** com busca e paleta de comandos (⌘K).
- **Sliders:** arrastar o valor, ⌥ para ajuste fino, duplo clique para voltar ao padrão, digitação direta do número.
- **Desfazer e refazer** ilimitado (⌘Z / ⇧⌘Z).
- **Atalhos de teclado** em todas as ações, **customizáveis** nas configurações.
- **Ícone:** "E" amarelo com textura de dither sobre fundo preto.

### Configurações
| Opção | Valores | Padrão |
|---|---|---|
| Idioma | English, Português, Seguir sistema | **English** |
| Tema | Escuro, Claro, Seguir sistema | **Escuro** |
| Atalhos | Lista editável | Padrões do app |
| Export | Formato, qualidade e escala padrão | PNG, 1x |

---

## 8. Arquitetura (resumo técnico)

- **Motor de render em grafo:** cada layer recebe uma imagem e devolve outra. **Cache por layer**, então editar um layer reprocessa só dele para cima.
- **GPU (Metal / Core Image com kernels próprios)** para quase tudo.
- **Difusão de erro** (sequencial) roda **na CPU com SIMD, em segundo plano**. No preview, a resolução é reduzida e a atualização é assíncrona.
- **Efeitos como módulos declarativos:** cada um define nome, categoria, parâmetros (tipo, faixa, padrão) e processamento. O inspector é gerado automaticamente. Adicionar efeito novo é barato.
- **Parâmetros relativos à resolução**, o que garante que o preview seja igual ao export.
- **Gestão de cor:** processamento linear, saída em sRGB.
- **Export em tiles**, para não estourar a memória da GPU.

---

## 9. Entrega

**Decisão:** entrega única com tudo do documento. Por dentro, o desenvolvimento segue em etapas, cada uma compilada, testada pelo Claude e com commit próprio:

1. Base: janela, abrir (incluindo RAW e ⌘V), canvas, layers e grupos, ajustes básicos, antes/depois, export, projeto, configurações
2. Dithering + paletas, grain, gradient map, halftone
3. Glow/bloom/halation, film emulation + LUT, trail/ghosting
4. VHS, ASCII, blob tracker
5. Presets (incluindo os de fábrica), crop, atalhos customizáveis, ícone e polimento

Ao final, o app é instalado em `/Applications` e o usuário testa tudo e manda feedback para a próxima atualização.

---

## 9.1 Extras implementados além do escopo
- Efeitos extras: Inverter, Posterizar, Limiar (Threshold), Desfoque (Blur), Ajustes Básicos completos.
- Paleta de comandos (⌘K) para buscar efeitos, presets e comandos.
- Modo Solo por layer; miniaturas dos efeitos e presets geradas a partir da imagem aberta.
- 18 presets de fábrica.

## 9.2 v0.2.0 (feedback do primeiro teste)
**Correções e UX**
- Campo de zoom com mais respiro; tecla **Z** ajusta à janela; lixeira em cada layer; botão de configurações na barra; cabeçalho com ícone, nome e versão.
- **Menus suspensos novos (pilar de UX):** clicar + ↑↓ com prévia ao vivo, ↩ mantém, ⎋ desfaz; seções agrupadas; favoritos (★) no topo; toda a navegação conta como um único passo de desfazer.
- HSL e Matiz/Saturação refeitos em OKLCh (perceptual), com mapeamento de gamut: sem artefatos ao escurecer cores, e os tons neutros não são afetados. O HSL ganhou bolinhas de cor nas seções e sliders com trilho colorido.
- Halftone com supersampling (sem moiré ao mudar o tamanho da célula); ASCII e linhas de varredura usam pixels inteiros; sliders com passos inteiros onde faz sentido.
- Sliders recalibrados com curva de resposta (mais controle no começo da escala): grain, raios, tamanhos, blob tracker.
- LUT: arrastar e soltar `.cube`, biblioteca de LUTs salvas (Application Support/EFFECTOR/LUTs).
- Navegador de efeitos com a altura toda da janela, modo lista com descrições.
- Salvar a pilha inteira como preset (botão nos Layers e nos Presets).
- Blob tracker: escala da caixa, caixa de tamanho fixo e padding negativo.
- 7 modos de mesclagem novos (Subexposição/Superexposição de Cor, Adição, Superexposição Linear, Exclusão, Matiz, Saturação).

**VHS refeito**: luma com banda limitada + realce de bordas de videocassete (halos), croma de baixa resolução com atraso e média entre linhas, ruído de croma, troca de cabeçote na base, pretos levantados, roll-off de altas luzes, tom da fita, OSD com fonte larga e contorno; datas de filmadora gravadas na fita (degradam junto). 11 Looks: Fita SP Limpa, Vídeo Caseiro '86, Videoclipe '89, Locadora '94, Cópia de 3ª Geração, TV de Madrugada, Fita Mastigada, Filmadora '98, Férias em Hi8, MiniDV 2003, CFTV.

**Novos efeitos (13):** Nitidez, Fotocópia, Cupom Térmico, Vazamento de Luz, Aberração Cromática, Vinheta, Digicam 2003 (JPEG real), Poeira & Arranhões, Papel (fibras, amassado em facetas, dobras, manchas, bordas), Plástico Filme, Tela CRT (máscaras de fósforo), Artefatos de JPEG (recompressão real), Moldura & Borda (Polaroid, Instax, tira 35 mm, slide, borda).

**Mais filmes (40 no total)**, agrupados: Negativo Colorido, Slide, Cinema, Instantâneo, Criativo (Cross Process, LomoChrome Purple, Aerochrome com matriz de canais) e P&B (todos exatamente neutros, verificado).

**Mais paletas e degradês**, agrupados por tema. **56 presets de fábrica** em 7 categorias, incluindo Receipt, Receipt Scan, Xerox, Zine, Fax, Y2K Blue, Chrome Dream, Frutiger Aero, Digicam 2003, Flip Phone, Deep Fried, VHS por época, Polaroid, Instax, Folha de Contato.

## 10. Backlog futuro (não fazer agora)
- Importar paletas .ase (Adobe)
- Pixel sort e outros glitches
- Versão para iPad (ver resposta sobre portabilidade na conversa de 24/11/2026)
- Máscaras (luminância, gradiente, pincel)
- Várias imagens abertas
- Lote
- Export SVG (dither/halftone)
- Vídeo e animação
- Remover os presets de fábrica, se o usuário quiser
