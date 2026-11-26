# EFFECTOR: Guia Rápido

## Abrir imagens
- **⌘O** (pode escolher várias), arrastar arquivos para a janela ou **⌘V** para colar. Cada imagem entra na **filmstrip** embaixo da tela, com os próprios efeitos, recorte, canvas e histórico de desfazer.
- **← / →** trocam de imagem. **F6** mostra ou esconde a filmstrip. **⌘W** fecha a imagem atual. A rodinha do mouse (sem Shift) rola a filmstrip.
- **⌫ (Delete)**: com fotos selecionadas na filmstrip (último clique nela), fecha essas fotos; pergunta antes se tiverem alterações. Fora da filmstrip, exclui os layers selecionados.
- Na filmstrip: clique abre; **⌘-clique** ou **⇧-clique** seleciona várias. O botão **⟳** (ou **⇧⌘S**, como no Lightroom) **sincroniza** os ajustes da imagem ativa nas selecionadas (layers — substituindo ou por cima —, recorte e canvas). Clique direito: copiar/colar ajustes, limpar, mostrar no Finder, fechar. **⇧⌘C / ⇧⌘V** copiam e colam os ajustes, **⇧⌘R** redefine, **⌘A** (depois de clicar na filmstrip) seleciona todas.
- **Exportar** com várias imagens: escolha "Esta imagem", "Selecionadas" ou "Todas"; cada uma sai com os próprios efeitos, com o nome montado pelo modelo de nomes do export.
- Formatos aceitos: JPG, PNG, HEIC, TIFF, RAW (CR3, ARW, NEF, DNG…) e projetos `.effector`.

## Layers
- **Adicionar Efeito** (ou **⇧⌘A**) abre o navegador de efeitos na **metade de baixo da sidebar**, no lugar dos presets (que voltam quando você fecha os efeitos), com miniaturas feitas a partir da sua imagem.
- **Prévia ao vivo:** passe o mouse sobre um efeito ou preset (ou use ↑ ↓ ← →) para vê-lo aplicado na imagem antes de adicionar. **↩** adiciona, **⎋** fecha. Dá para desligar em Configurações.
- **Seleção múltipla:** **⇧-clique** seleciona um intervalo, **⌘-clique** adiciona ou remove um layer, **⌥⌘A** seleciona todos. Com vários selecionados aparecem ações em massa (agrupar, duplicar, ocultar, excluir), e a barra de mesclagem e opacidade muda todos de uma vez. Arrastar qualquer layer selecionado move todos.
- **⌘K** abre a paleta de comandos: digite o nome de qualquer efeito, preset ou comando.
- O botão no topo do navegador alterna entre **grade** e **lista** (a lista é compacta: só miniatura pequena e nome; a descrição aparece ao passar o mouse). Um título discreto no topo mostra se o painel aberto é **Adicionar Efeito** ou **Presets**.
- O layer **de cima** é aplicado **por último**, sobre tudo o que está abaixo.
- Arraste para reordenar. Soltar no meio de um grupo coloca o layer **dentro** dele.
- **⌘G** agrupa e **⇧⌘G** desagrupa. **⌘J** duplica (como no Photoshop) e **⌫** exclui. Cada grupo tem a própria opacidade e o próprio modo de mesclagem.
- Olho = mostrar/ocultar (**H**). Lixeira (ao lado do olho) = excluir.
- **Mesclagem e opacidade** ficam no topo da lista de layers, como no Photoshop, e valem para o layer selecionado (ou todos os selecionados). Na opacidade: clique ou arraste para ajustar, role o mouse para ajuste fino e dê duplo clique para 100%.
- **Ícone de máscara** em cada layer, sempre visível ao lado da lixeira e do olho (fica amarelo quando o layer tem máscara): clique para escolher uma máscara da imagem, inverter, criar uma nova ou editar.
- O botão de salvar (no topo dos layers) guarda **a pilha inteira como preset**: ordem, grupos, parâmetros, opacidade e mesclagem.
- Duplo clique no nome renomeia. Clique direito abre mais opções, como "Salvar como Preset".

## Controles
- Arraste o slider; segure **⌥** para ajuste fino.
- **Duplo clique** no slider volta ao valor padrão.
- Clique no número para digitar um valor exato.
- O botão ↺ no topo do inspector redefine todos os parâmetros do layer. O **dado** (ou **⇧R**) sorteia novos valores para explorar.
- **Interruptores nas seções:** cada parte de um efeito (grão, halation, rótulos do blob tracker, borda preta do CRT, ruído, data…) tem um liga/desliga no título da seção.
- **Copiar / colar layers:** ⌥⌘C e ⌥⌘V (funciona entre imagens diferentes).
- **Duplo clique** em qualquer slider (ou no nome dele) volta ao valor padrão.
- **Menus suspensos** (filmes, degradês, paletas, mesclagem, formatos, LUTs, Looks): clique e use **↑ ↓** ou **W / S** para ir passando com prévia ao vivo. **↩** mantém, **⎋** volta ao que estava antes. Com o menu fechado mas selecionado, ↑ ↓ também trocam a opção. A estrela ★ favorita: os favoritos aparecem no topo.
- **Look**: todos os efeitos têm um seletor de Look no topo com pontos de partida prontos (de 3 a 20 por efeito).
- **Paleta**: efeitos com cores escolhíveis (Light Leak, Threshold, Dither, Halftone, LED, Trail, Neblina…) têm um seletor de **Paleta** que troca todas as cores de uma vez, com combinações prontas para cada efeito.
- **LUT**: arraste arquivos `.cube` para o campo da LUT (ou direto para a imagem, que cria um layer de LUT). Toda LUT carregada fica salva na sua biblioteca e aparece no menu.

## Presets
- Botão **Presets** (ou **P**): abre os presets na **metade de baixo da sidebar**, dividindo o espaço com os layers (arraste a divisória para mudar a proporção; duplo clique volta para a metade). O painel fica aberto depois de aplicar e só fecha no **✕** ou no **P**; o app lembra se estava aberto. As setas navegam nos presets enquanto o mouse está sobre o painel ou a busca está ativa. Clique num preset para adicioná-lo **por cima como grupo**. São 100 presets do app, separados em DM's Specials, Cinema, Filme, Retrato & Luz, Y2K & Digital, Vídeo Retrô, Impressão & Gráfico, Pixel & Código, Filtros de Lente e Rastreio & HUD. Cada um combina de 3 a 8 efeitos (alguns organizados em grupos, como "Grade" e "Lens"), com grão fino e discreto.
- O botão **+** salva a pilha atual como preset.
- Menu ⋯: salvar a pilha atual, importar e exportar (`.effectorpreset`), mostrar ou ocultar os presets do app.

## Janela
- No topo fica o nome do app e a versão. Na **barra fina embaixo da imagem** ficam o nome do arquivo, o tamanho, a proporção, os megapixels, o tipo, o número de layers, se há alterações não salvas e o **controle de zoom** (− % +, ajustar e tela cheia).
- **F** abre a imagem em **tela cheia**, sem painéis. **F** ou **⎋** volta.
- **Y** liga a comparação **Antes / Depois dividida** (como no Lightroom): arraste a divisória para comparar.
- **Tab** esconde as barras laterais; **⇧Tab** esconde todos os painéis.
- **Clique direito fora da imagem** escolhe a cor de fundo da área central: branco, quatro cinzas ou preto (a linha inteira de cada opção é clicável).
- As duas barras laterais são **redimensionáveis**: arraste a divisória. Duplo clique na divisória volta à largura padrão.
- Passe o mouse sobre qualquer botão para ver o nome e o atalho.
- O app **reabre a última imagem com todos os layers** ao ser aberto de novo (dá para desligar em Configurações). Configurações, atalhos, favoritos, presets, paletas, degradês e LUTs são sempre salvos.

## Visualização
- **]** ou o botão na barra alterna **Antes / Depois**.
- **Monitoramento** no topo da sidebar da direita, com duas abas. **Histograma**: RGB, Luminância, R, G, B ou Tudo. **Vectorscope**: as cores da imagem por matiz e saturação, cada ponto na sua cor, com alvos de R, Mg, B, Ci, G e Am e a **linha de tom de pele** (zoom 1× / 2×). Clique no título (ou ⇧⌘H) para recolher.
- **J** mostra o **recorte** na imagem: altas luzes estouradas em vermelho, sombras esmagadas em azul. Os triângulos do histograma também ligam e desligam.
- Pinça ou **⌘ + rolagem** para zoom. Arraste ou role para mover a imagem: isso só funciona quando a imagem é maior que a área de visualização (no eixo em que ela passa da tela). Ajustada à janela, ela não sai do lugar.
- Duplo clique na imagem ajusta à janela.
- **Z** alterna entre ajustar à janela e 100% (como no Lightroom); **⌘0** ajusta e **⌘1** mostra em 100%.

## Ferramentas (fileira de ícones da direita)
- Embaixo do histograma há uma fileira de ícones, como no Lightroom Classic: **Editar** (**E**: os controles do layer selecionado), **Recorte** (**R**), **Canvas & Bordas** (**⇧B**) e **Máscaras** (**⇧W**). Apertar o atalho de novo volta para Editar. Um pontinho amarelo marca as ferramentas que estão mudando a imagem (recorte aplicado, canvas ligado, máscaras criadas).

## Canvas & Bordas (Instagram)
- Ícone **Canvas & Bordas** na fileira de ferramentas (ou **⇧B**): encaixa a imagem numa proporção — 4:5, 1:1, 9:16, 3:4, 1.91:1, 16:9, impressão, A4 ou personalizada — com borda mínima, fundo de **cor**, **degradê** ou a **própria foto desfocada**, posição, cantos arredondados e sombra. O export sai no tamanho do canvas.

## Recorte
- **R** (como no Lightroom) ou o ícone de recorte na fileira de ferramentas. Escolha a proporção (Livre, 1:1, 4:5, 16:9… ou **Personalizada**, digitando por exemplo "2,39x1", "4:5" ou "16/9"; as últimas ficam em Recentes) no painel da direita e clique em **Concluir** (ou **↩**). **⎋** cancela. Trocar de ferramenta também aplica o recorte, como no Lightroom.

## Máscaras
- As máscaras são da **imagem**, não de cada efeito: crie uma vez e use em quantos efeitos quiser. Ícone **Máscaras** (ou **⇧W**) → **Criar Nova Máscara**, ou direto pelos atalhos do Lightroom Classic:
  - **Selecionar Assunto** (**⇧S**) e **Selecionar Fundo** (**⇧G**): a IA encontra o assunto principal da foto no próprio computador (cerca de 2 s na primeira vez em cada foto). Ajustes: **Suavidade da Borda** e **Expandir / Contrair**.
  - **Pincel** (**K**): pinte na imagem. **[ ]** mudam o tamanho, **⇧[ ⇧]** a suavidade; ou segure **⌘** (Mac) / **Alt** (Windows) e arraste com o **botão direito**: para os lados muda o tamanho, para cima/baixo a suavidade. Segure **⌥** (Alt) para apagar, ou ligue **Apagar**. **Fluxo** controla quanto cada pincelada acumula. **⎋** ou **Concluído** para de pintar.
  - **Gradiente Linear** (**M**): arraste na imagem de onde o efeito é total até onde ele some.
  - **Gradiente Radial** (**⇧M**): arraste do centro para fora; segure **⇧** para um círculo. Ajustes: tamanho, arredondamento, suavidade e ângulo.
- Gradientes mostram **alfinete e alças** na imagem: arraste o alfinete (ou a linha pontilhada do meio do linear) para mover. No **anel em volta do alfinete** o cursor vira uma seta de girar: arraste para **rotacionar**. No linear, puxar uma das **linhas sólidas** mexe só aquele lado (a suavidade). No radial, as bolinhas mudam o tamanho e a **elipse pontilhada** muda a suavidade.
- **⇧ em qualquer arraste:** gradiente linear reto (em passos de 45°), radial em círculo perfeito, mover só na horizontal ou na vertical, girar em passos de 15°.
- Cada máscara tem partes: a primeira é a **principal** (borda amarela); as outras aparecem recuadas embaixo dela, ligadas por uma linha; **Adicionar** (verde, **+**) e **Subtrair** (vermelho, **−**) acrescentam outras partes de qualquer tipo. Cada parte mostra a cor e o sinal do modo (verde soma, vermelho tira, azul **∩** fica só com o que as duas têm em comum). A sobreposição vermelha mostra só o resultado final (o que foi subtraído fica vazado). Cada parte tem inverter (**'**, como no Lightroom), olho e remover (**⌫** remove a parte selecionada).
- **Efeitos que Usam Esta Máscara**: marque os efeitos que a máscara limita, ou use o ícone de máscara de cada layer.
- **Mostrar Sobreposição** (**O**) pinta de vermelho a área da máscara selecionada.
- **↩ ou ⎋** voltam para Editar (com o pincel ou um gradiente armado, o primeiro ⎋ só para de desenhar). O mesmo vale para Canvas & Bordas; no Recorte, ↩ aplica e ⎋ cancela.
- Projetos e presets antigos com máscaras de profundidade abrem com máscaras de Assunto ou Fundo no lugar. Presets e layers copiados levam as máscaras junto para outras imagens.

## Salvar e exportar
- **⌘S** salva o projeto `.effector` (imagem original + todos os layers editáveis).
- **⇧⌘E** abre o **Export**, no estilo do Lightroom:
  - **Local**: pasta específica, mesma pasta do original ou escolher na hora; **subpasta** opcional; o que fazer com **arquivos que já existem** (perguntar, nome novo, sobrescrever ou pular).
  - **Nome dos arquivos**: modelos prontos no menu ou um modelo montado com **blocos** (nome do arquivo, texto livre, sequência, data da exportação, data da foto, dimensões, separadores e texto). Clique num bloco para trocar o formato, mover ou remover. O **Exemplo** mostra o nome final.
  - **Arquivo**: JPG (qualidade e **limitar tamanho do arquivo**) ou PNG (mantém a transparência).
  - **Tamanho**: redimensionar para caber em largura e altura, dimensões, lado maior, lado menor, megapixels ou porcentagem, em pixels, polegadas ou centímetros, com **Não Ampliar**.
  - **Depois de exportar**: não fazer nada ou mostrar no Explorer / Finder.
  - **Presets de Exportação** na lista da esquerda: **+** salva as configurações atuais; clique direito atualiza, renomeia ou apaga. A lista começa vazia.
- **⌥⇧⌘C** copia o resultado para colar em outro app.

## Configurações (⌘, ou a engrenagem na barra)
- Idioma (English / Português / Sistema), tema (Escuro / Claro / Sistema) e atalhos personalizáveis. As configurações do export ficam na própria janela de export.
- **Atualizações**: **Verificar Atualizações** procura uma versão nova no GitHub, baixa e instala (o app fecha, atualiza e abre de novo). O app também verifica **toda vez que abre** e, se houver versão nova, mostra um aviso com as novidades e os botões **Baixar e Instalar**, **Depois** e **Pular Esta Versão** (dá para desligar a verificação automática).

## Todos os atalhos (padrão, personalizáveis)
Baseados no **Lightroom Classic** quando a função existe lá, e no **Photoshop** para layers. No Windows, ⌘ = Ctrl e ⌥ = Alt.

| Ação | Atalho |
|---|---|
| Abrir | ⌘O |
| Salvar / Salvar como | ⌘S / ⌥⌘S |
| Exportar | ⇧⌘E |
| Copiar resultado | ⌥⇧⌘C |
| Colar imagem | ⌘V |
| Desfazer / Refazer | ⌘Z / ⇧⌘Z |
| Paleta de comandos | ⌘K |
| Adicionar efeito | ⇧⌘A |
| Antes / Depois | ] |
| Antes / Depois dividido | Y |
| Ferramentas: Editar / Recortar / Canvas & Bordas / Máscaras | E / R / ⇧B / ⇧W |
| Sobreposição da máscara | O |
| Máscara: Pincel / Gradiente Linear / Gradiente Radial | K / M / ⇧M |
| Máscara: Selecionar Assunto / Selecionar Fundo | ⇧S / ⇧G |
| Pincel: tamanho / suavidade | [ ] / ⇧[ ⇧] |
| Inverter / remover a parte da máscara | ' / ⌫ |
| Mostrar recorte | J |
| Mostrar / ocultar histograma | ⇧⌘H |
| Agrupar / Desagrupar | ⌘G / ⇧⌘G |
| Duplicar layer | ⌘J |
| Excluir layers (ou fechar fotos na filmstrip) | ⌫ |
| Mostrar / ocultar layer | H |
| Mover layer para cima / baixo | ⌘] / ⌘[ |
| Selecionar tudo / Desmarcar | ⌘A / ⌘D |
| Selecionar todos os layers | ⌥⌘A |
| Copiar / colar / redefinir ajustes | ⇧⌘C / ⇧⌘V / ⇧⌘R |
| Sincronizar ajustes | ⇧⌘S |
| Copiar / colar layers | ⌥⌘C / ⌥⌘V |
| Zoom + / − | ⌘= / ⌘- |
| Ajustar ↔ 100% / 100% | Z / ⌘1 |
| Filmstrip | F6 |
| Esconder barras laterais / todos os painéis | Tab / ⇧Tab |
| Fechar painéis / Presets | L / P |
| Tela cheia | F |
| Fechar imagem | ⌘W |
| Aleatorizar layer | ⇧R |

## Glow × Bloom × Halation
- **Glow**: difusão suave da imagem inteira acima de um limiar baixo. Dá aquela névoa sonhadora (filtro de difusão).
- **Bloom**: só as **luzes mais fortes** vazam, em vários tamanhos ao mesmo tempo, como a luz estourando numa lente. As sombras e os meios-tons ficam intactos.
- **Halation**: imita a luz rebatendo na base do filme e reexpondo a camada vermelha. O centro das luzes continua branco e a franja vai do **laranja** (perto da luz) ao **vermelho** (mais longe). *Espalhamento* equilibra franja justa × halo largo, *Origem* escolhe entre a camada vermelha (fiel ao filme: luzes azuis não fazem halo) e todas as luzes fortes, e *Bloom* soma um brilho neutro. Desde a 0.7 o Film Emulation não tem mais halation embutido: use uma camada de Halation embaixo dele.

## Novos na 0.7
- **Split Tone**: uma cor nas sombras e outra nas altas luzes (ex.: sombras azuladas, luzes quentes).
- **Vidro Canelado**: a imagem atrás de um vidro com frisos verticais (largura, refração, curvatura, dispersão, "derretido").
- **Painel de LED**: a imagem refeita com LEDs (tamanho, altura do ponto — pontos altos viram barras —, limiar, níveis, brilho).
- **Desgaste de Impressão**: toner desbotado, riscos do cilindro, faixas, linhas mortas do cabeçote, sujeira e sombra nas bordas, sobre qualquer imagem.
- **Fotocópia** e **Cupom Térmico** viraram presets (em *Print & Graphic*), montados com Threshold/Dither + Desgaste de Impressão. Projetos antigos que usam os efeitos continuam abrindo.
- Categoria de presets **DM'S SPECIALS** (17 looks) no topo da lista.

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

## Desfoque
- **Desfoque** (gaussiano ou "superfície", que preserva bordas), **Desfoque Radial** (zoom/giro com centro nítido, como no Resolve), **Desfoque de Movimento** e **Desfoque de Lente & Tilt-Shift**. Desfoque e Movimento podem atuar em tudo, só nas bordas ou só no centro. Olho de Peixe e Vinheta também têm desfoque de borda.
- **Distorção de Borda**: mantém o centro intacto e distorce só as bordas (curvar, esticar, redemoinho, ondulação, líquido, comprimir).

## Novos na 0.8
- **Mapa de profundidade (IA no próprio computador)**: o app estima o que está perto e longe em cada foto (cerca de 1 s, só quando algum layer precisa). **D** mostra o mapa.
- **Máscara de Profundidade** em qualquer layer: limite o efeito a uma faixa de distância (Fundo, Meio, Assunto ou personalizado), com suavidade e inverter.
- **Onde (Profundidade)**: Glow, Bloom, Halation, Light Leak, Grão, Estrela, Anamórfico, Prisma, Trail, Camcorder e Color Grading podem ficar mais fortes no fundo ou no primeiro plano.
- **Desfoque de Profundidade**: bokeh de verdade (redondo, de lâminas, oval anamórfico, espiral, olho de gato). O **conta-gotas** "Escolher na imagem" define o foco clicando na foto.
- **Emulação de Lentes**: lentes inteiras (Cinema Prime, Anamórfica 2x, Helios 44, Petzval, Zoom anos 70, Black Pro-Mist, câmera de brinquedo…).
- **Neblina e Névoa** (engrossa com a distância) e **Paralaxe e Relevo** (experimental).
- **Camcorder** substitui VHS e Digicam: lente de filmadora, cor de CCD, contornos brancos, NightShot, fita, TV filmada, compressão e **carimbo de data** que vem da data da foto (editável).
- **Color Grading** (substitui o Split Tone): rodas de cor para sombras, meios-tons, altas luzes e global, com Mescla e Equilíbrio.
- **Prisma**: novos tipos Prisma Linear (degraus/ecos) e Meio Prisma (dioptria dividida); o **Cristal** virou um filtro de facetas de verdade.
- **Trail**: trajetos em arco, onda, tremido de mão e espiral, cópias em arco-íris ou degradê e rastro só das luzes.
- **Estrela** e **Anamórfico**: controles para afetar só as luzes mais fortes, densidade e intensidade bem sutil; novo filtro de riscos.
- **Vidro Canelado** refeito com base em vidro real; **Poeira** bem mais fina e densa; **Halo** removido.
- Sliders revisados: números limpos, passo fino perto do zero (o Min Size do Blob Tracker não pula mais), curvas melhores.

## Novos na 0.8.1
- **Color Grading** ganhou o modo **Split Tone** (o antigo efeito Split Tone, que saiu do menu). Projetos antigos são convertidos sozinhos.
- **Fonte** em todo efeito com texto (Blob Tracker, VHS, Camcorder, Digicam, Moldura, ASCII), com 10 fontes novas no estilo web dos anos 2000: VT323, Silkscreen, Press Start 2P, DotGothic16, Share Tech Mono, Space Mono, IBM Plex Mono, Major Mono Display, Courier Prime e Cutive Mono.
- O Blob Tracker exporta exatamente as caixas do preview.

## Novos na 0.8.2
- **Fileira de ferramentas** na direita (Editar, Recorte, Canvas & Bordas, Máscaras) e **máscaras como objetos da imagem**, usadas por vários efeitos.
- **Detalhe** (antigo Nitidez): seção **Presença** com **Textura**, **Claridade** e **Remover Névoa**, além da nitidez.
- **Ajustes Básicos** na ordem do Lightroom: Balanço de Branco (temperatura, matiz), Tom (exposição, contraste, altas luzes, sombras, brancos, pretos) e Cor (vibração, saturação).
- **HSL**: botão **Por Cor / Por Ajuste**; Por Ajuste mostra uma seção de Matiz, uma de Saturação e uma de Luminância com todas as cores.
- **Export** novo, com presets de exportação.
- Aviso de **atualização** ao abrir o app.
- Dithering, halftone e ASCII **sem padrões falsos** quando o zoom está abaixo de 100%.
- Presets com **Moldura** vêm com ela **desligada** (olho fechado): ligue o layer para usar a borda.

## Novos na 0.9.0
- **Máscaras de verdade**: Pincel (**K**), Gradiente Linear (**M**), Gradiente Radial (**⇧M**), Selecionar Assunto (**⇧S**) e Selecionar Fundo (**⇧G**), desenhadas e ajustadas direto na imagem.
- **Detecção de assunto por IA** no lugar do mapa de profundidade: Desfoque de Fundo (antigo Desfoque de Profundidade), Emulação de Lentes e Neblina usam o assunto, e a seção **Onde** dos efeitos virou **Aplicar Em: Em tudo / Fundo / Assunto**. Paralaxe & Relevo saiu.
- **Mesclagem, opacidade e máscara** na lista de layers, como no Photoshop (o botão Solo saiu).
- **Monitoramento** com Histograma e **Vectorscope** (linha de tom de pele).
- **Proporção de recorte personalizada** ("2,39x1", "4:5"…).
- **Filtros de Lente** (categoria nova de presets, no lugar de Sonho & Surreal) com dois efeitos novos: **Filtro de Difusão** (Black Pro-Mist, Pro-Mist, Cinebloom, Glimmerglass, Pearlescent, Hollywood Black Magic, Black Satin, Soft Focus e Fog, em graduações 1/8 a 2) e **Filtro Fantasma** (cópias deslocadas e rastros de obturador lento, de um lado, dos dois, nos cantos ou em volta do centro).
