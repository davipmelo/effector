# EFFECTOR: Guia Rápido

## Abrir uma imagem
- **⌘O**, arrastar o arquivo para a janela ou **⌘V** para colar da área de transferência.
- Formatos aceitos: JPG, PNG, HEIC, TIFF, RAW (CR3, ARW, NEF, DNG…) e projetos `.effector`.

## Layers
- **Adicionar Efeito** (ou **⇧⌘A**) abre o navegador de efeitos ao lado dos layers, com miniaturas feitas a partir da sua imagem.
- **Prévia ao vivo:** passe o mouse sobre um efeito ou preset (ou use ↑ ↓ ← →) para vê-lo aplicado na imagem antes de adicionar. **↩** adiciona, **⎋** fecha. Dá para desligar em Configurações.
- **Seleção múltipla:** **⇧-clique** seleciona um intervalo, **⌘-clique** adiciona ou remove um layer, **⌥⌘A** seleciona todos. Com vários selecionados aparecem ações em massa (agrupar, duplicar, ocultar, excluir), e o inspector muda opacidade e mesclagem de todos de uma vez. Arrastar qualquer layer selecionado move todos.
- **⌘K** abre a paleta de comandos: digite o nome de qualquer efeito, preset ou comando.
- O navegador ocupa a altura toda da janela. O botão no topo alterna entre **grade** e **lista** (a lista mostra uma descrição de cada efeito).
- O layer **de cima** é aplicado **por último**, sobre tudo o que está abaixo.
- Arraste para reordenar. Soltar no meio de um grupo coloca o layer **dentro** dele.
- **⌘G** agrupa e **⇧⌘G** desagrupa. Cada grupo tem a própria opacidade e o próprio modo de mesclagem.
- Olho = mostrar/ocultar (**H**). Lixeira (ao lado do olho) = excluir. **S** (ao passar o mouse) = solo: mostra só aquele layer.
- O botão de salvar (no topo dos layers) guarda **a pilha inteira como preset**: ordem, grupos, parâmetros, opacidade e mesclagem.
- Duplo clique no nome renomeia. Clique direito abre mais opções, como "Salvar como Preset".

## Controles
- Arraste o slider; segure **⌥** para ajuste fino.
- **Duplo clique** no slider volta ao valor padrão.
- Clique no número para digitar um valor exato.
- O botão ↺ no topo do inspector redefine todos os parâmetros do layer. O **dado** (ou **R**) sorteia novos valores para explorar.
- **Interruptores nas seções:** cada parte de um efeito (grão, halation, rótulos do blob tracker, borda preta do CRT, ruído, data…) tem um liga/desliga no título da seção.
- **Copiar / colar layers:** ⌥⌘C e ⌥⌘V (funciona entre imagens diferentes).
- **Duplo clique** em qualquer slider (ou no nome dele) volta ao valor padrão.
- **Menus suspensos** (filmes, degradês, paletas, mesclagem, formatos, LUTs, Looks): clique e use **↑ ↓** ou **W / S** para ir passando com prévia ao vivo. **↩** mantém, **⎋** volta ao que estava antes. Com o menu fechado mas selecionado, ↑ ↓ também trocam a opção. A estrela ★ favorita: os favoritos aparecem no topo.
- **Look**: vários efeitos (VHS, CRT, Papel, Poeira, Vazamento de Luz, Digicam) têm um seletor de Look no topo com pontos de partida prontos.
- **LUT**: arraste arquivos `.cube` para o campo da LUT (ou direto para a imagem, que cria um layer de LUT). Toda LUT carregada fica salva na sua biblioteca e aparece no menu.

## Presets
- Botão **Presets** (ou **P**): abre o painel ao lado dos layers, sem escondê-los. Clique num preset para adicioná-lo **por cima como grupo**. Os presets estão separados por categoria (Filme, Y2K & Digital, Vídeo Analógico, Impressão & Papel, Dither & Pixel, Luz & Clima, Rastreio).
- O botão **+** salva a pilha atual como preset.
- Menu ⋯: salvar a pilha atual, importar e exportar (`.effectorpreset`), mostrar ou ocultar os presets do app.

## Janela
- No topo fica o nome do app e a versão. Na **barra fina embaixo da imagem** ficam o nome do arquivo, o tamanho, a proporção, os megapixels, o tipo, o número de layers, se há alterações não salvas e o **controle de zoom** (− % +, ajustar e tela cheia).
- **F** abre a imagem em **tela cheia**, sem painéis. **F** ou **⎋** volta.
- **/** liga a comparação **Antes / Depois dividida**: arraste a divisória para comparar.
- As duas barras laterais são **redimensionáveis**: arraste a divisória. Duplo clique na divisória volta à largura padrão.
- Passe o mouse sobre qualquer botão para ver o nome e o atalho.
- O app **reabre a última imagem com todos os layers** ao ser aberto de novo (dá para desligar em Configurações). Configurações, atalhos, favoritos, presets, paletas, degradês e LUTs são sempre salvos.

## Visualização
- **\\** (barra invertida) ou o botão na barra alterna **Antes / Depois**.
- Pinça ou **⌘ + rolagem** para zoom. Arraste ou role para mover a imagem.
- Duplo clique na imagem ajusta à janela.
- **Z** (ou **⌘0**) ajusta à janela e **⌘1** mostra em 100%.

## Recorte
- **C** ou o botão ✂︎ abre o recorte. Escolha a proporção (Livre, 1:1, 4:5, 16:9…) e clique em **Aplicar**.

## Salvar e exportar
- **⌘S** salva o projeto `.effector` (imagem original + todos os layers editáveis).
- **⌘E** exporta em PNG ou JPG, em 1× ou 2×. PNG mantém a transparência.
- **⇧⌘C** copia o resultado para colar em outro app.

## Configurações (⌘, ou a engrenagem na barra)
- Idioma (English / Português / Sistema), tema (Escuro / Claro / Sistema), atalhos personalizáveis e padrões de exportação.

## Todos os atalhos (padrão, personalizáveis)
| Ação | Atalho |
|---|---|
| Abrir | ⌘O |
| Salvar / Salvar como | ⌘S / ⇧⌘S |
| Exportar | ⌘E |
| Copiar resultado | ⇧⌘C |
| Colar imagem | ⌘V |
| Desfazer / Refazer | ⌘Z / ⇧⌘Z |
| Paleta de comandos | ⌘K |
| Adicionar efeito | ⇧⌘A |
| Antes / Depois | \ |
| Recortar | C |
| Agrupar / Desagrupar | ⌘G / ⇧⌘G |
| Duplicar layer | ⌘D |
| Excluir layer | ⌘⌫ |
| Mostrar / ocultar layer | H |
| Mover layer para cima / baixo | ⌘] / ⌘[ |
| Zoom + / − | ⌘= / ⌘- |
| Ajustar à janela / 100% | Z (ou ⌘0) / ⌘1 |
| Fechar painéis / Presets | L / P |
| Selecionar todos os layers | ⌥⌘A |
| Tela cheia | F |
| Antes / Depois dividido | / |
| Aleatorizar layer | R |
| Copiar / colar layers | ⌥⌘C / ⌥⌘V |

## Glow × Bloom × Halation
- **Glow**: difusão suave da imagem inteira acima de um limiar baixo. Dá aquela névoa sonhadora (filtro de difusão).
- **Bloom**: só as **luzes mais fortes** vazam, em vários tamanhos ao mesmo tempo, como a luz estourando numa lente. As sombras e os meios-tons ficam intactos.
- **Halation**: um halo **vermelho-alaranjado** em volta das luzes, que imita a luz rebatendo na base do filme.

## Filtros de lente
- **Prisma & Caleidoscópio**: três tipos — Prisma (fantasmas com arco-íris), Caleidoscópio (segmentos, espelho, rotação, zoom, torção, anéis, origem) e Cristal (facetas). Veja os Looks: Mandala, Vórtice, Viagem Psicodélica, Cacos de Vidro…
- **Flare Anamórfico**: riscos horizontais nas luzes (azul clássico, âmbar, espectral).
- **Filtro Estrela**: estrelas de 4, 6 ou 8 pontas.
- **Prisma**: reflexos com arco-íris de um lado, nas bordas ou no quadro todo.
- **Halo**: anéis de luz em volta das luzes.
- **Desfoque de Lente & Tilt-Shift**: faixa ou centro nítido com bokeh.

## Distorção
- **Olho de Peixe** (barril ou olho de peixe real até 340°), **Redemoinho**, **Pinçar & Inflar**, **Ondas & Ondulações**, **Mini Planeta & Túnel**, **Líquido & Vidro**, **Espelho & Mosaico** e **Esticar Pixels**. As bordas são preenchidas espelhando, sem buracos pretos.

## Molduras
- **Faixas pretas (letterbox)** em proporções de cinema (1.33 a 3.00:1), com **Reenquadrar** e legenda opcional; Polaroid, borda simples e foto arredondada.
- O controle **Só Luzes Pequenas** (Anamórfico e Estrela) evita que áreas claras grandes estourem.
