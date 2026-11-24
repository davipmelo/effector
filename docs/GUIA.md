# EFFECTOR: Guia Rápido

## Abrir uma imagem
- **⌘O**, arrastar o arquivo para a janela ou **⌘V** para colar da área de transferência.
- Formatos aceitos: JPG, PNG, HEIC, TIFF, RAW (CR3, ARW, NEF, DNG…) e projetos `.effector`.

## Layers
- **Adicionar Efeito** (ou **⇧⌘A**) abre o navegador de efeitos, com miniaturas feitas a partir da sua imagem.
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
- O botão ↺ no topo do inspector redefine todos os parâmetros do layer.
- **Menus suspensos** (filmes, degradês, paletas, mesclagem, formatos, LUTs, Looks): clique e use **↑ ↓** para ir passando com prévia ao vivo. **↩** mantém, **⎋** volta ao que estava antes. Com o menu fechado mas selecionado, ↑ ↓ também trocam a opção. A estrela ★ favorita: os favoritos aparecem no topo.
- **Look**: vários efeitos (VHS, CRT, Papel, Poeira, Vazamento de Luz, Digicam) têm um seletor de Look no topo com pontos de partida prontos.
- **LUT**: arraste arquivos `.cube` para o campo da LUT (ou direto para a imagem, que cria um layer de LUT). Toda LUT carregada fica salva na sua biblioteca e aparece no menu.

## Presets
- Aba **Presets**: clique num preset para adicioná-lo **por cima como grupo**. Os presets estão separados por categoria (Filme, Y2K & Digital, Vídeo Analógico, Impressão & Papel, Dither & Pixel, Luz & Clima, Rastreio).
- O botão **+** salva a pilha atual como preset.
- Menu ⋯: salvar a pilha atual, importar e exportar (`.effectorpreset`), mostrar ou ocultar os presets do app.

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
| Aba Layers / Presets | L / P |

## Glow × Bloom × Halation
- **Glow**: difusão suave da imagem inteira acima de um limiar baixo. Dá aquela névoa sonhadora (filtro de difusão).
- **Bloom**: só as **luzes mais fortes** vazam, em vários tamanhos ao mesmo tempo, como a luz estourando numa lente. As sombras e os meios-tons ficam intactos.
- **Halation**: um halo **vermelho-alaranjado** em volta das luzes, que imita a luz rebatendo na base do filme.
