# EFFECTOR

Editor de efeitos para fotos em camadas, para **Windows** e **macOS**. Você empilha efeitos analógicos, digitais, de lente e de impressão sobre uma foto, ajusta cada um e exporta o resultado. A interface segue o Lightroom Classic e os atalhos também.

**Baixar:** [última versão em Releases](https://github.com/davipmelo/effector/releases/latest) (instalador para Windows, `.dmg` para Mac). O app avisa sozinho quando sai uma versão nova.

## Funções

### Edição em camadas
- Pilha de **layers** com grupos, opacidade e 18 modos de mesclagem no topo da lista (como no Photoshop), máscara em cada layer, mostrar / ocultar, duplicar, reordenar arrastando e seleção múltipla.
- **Looks** prontos em todo efeito, **paletas** de cores combinadas, degradês e paletas salvos, **LUTs** `.cube`.
- Prévia ao vivo ao passar o mouse sobre efeitos e presets; **paleta de comandos** (⌘K / Ctrl+K); desfazer ilimitado por imagem.

### Efeitos (48)
- **Ajuste & Cor:** Ajustes Básicos, Curvas, HSL (abas Matiz / Saturação / Luminância), Matiz / Saturação, Preto & Branco, Color Grading (rodas de cor e Split Tone), Mapa de Degradê, LUT, Detalhe (textura, claridade, remover névoa e nitidez), Inverter, Posterizar, Threshold.
- **Impressão & Pixel:** Dither, Halftone, ASCII, Desgaste de Impressão.
- **Filme & Luz:** Emulação de Filme, Grão, Glow, Bloom, Halation, Vazamento de Luz.
- **Filtros de Lente:** Emulação de Lentes, Filtro de Difusão (Black Pro-Mist, Pro-Mist, Cinebloom, Glimmerglass, Pearlescent, Hollywood Black Magic, Black Satin, Soft Focus, Fog), Filtro Fantasma, Flare Anamórfico, Filtro Estrela, Prisma & Caleidoscópio, Aberração Cromática, Vinheta.
- **Desfoque:** Desfoque, Desfoque de Fundo (bokeh atrás do assunto), Desfoque de Lente & Tilt-Shift, Desfoque de Movimento, Desfoque Radial.
- **Distorção:** Olho de Peixe, Distorção de Borda, Vidro Canelado, Espelho & Repetição, Pixel Stretch.
- **Glitch & Retrô:** Camcorder (VHS, Digicam, NightShot, carimbo de data), Tela CRT, Display de LED, Trail & Rastro.
- **Sobreposições:** Blob Tracker, Poeira & Arranhões, **Sobreposição de Imagem** (291 texturas do app em Poeira, Grunge, Vidro, Papel e Plástico, ou uma imagem sua, com encaixe, posição, rotação, distorção e tom; a imagem vai junto nos presets e projetos), **Molduras & Bordas** (letterbox, Polaroid, borda, cópia arredondada e **papel rasgado** com rasgo gerado por seed, lados e recuos independentes).

### Presets
- **100 presets** do app em 10 categorias (DM's Specials, Cinema, Filme, Retrato & Luz, Y2K & Digital, Vídeo Retrô, Impressão & Gráfico, Pixel & Código, Filtros de Lente, Rastreio & HUD), em grade ou lista compacta. Presets com moldura vêm com ela desligada.
- Presets próprios: salvar a pilha, renomear, importar e exportar (`.effectorpreset`).

### Ferramentas (fileira de ícones da direita)
- **Editar:** os controles do layer selecionado.
- **Recorte** com proporções prontas ou personalizadas ("2,39x1").
- **Canvas & Bordas:** encaixa a foto em outra proporção (Instagram, impressão…) com borda de cor, degradê ou foto desfocada, cantos arredondados e sombra.
- **Máscaras** (atalhos do Lightroom Classic): **Pincel** (K), **Gradiente Linear** (M), **Gradiente Radial** (⇧M), **Selecionar Assunto** (⇧S) e **Selecionar Fundo** (⇧G), desenhadas e ajustadas na imagem, com partes que somam, subtraem ou se cruzam e sobreposição vermelha (O). Uma máscara pode limitar vários efeitos.

### Assunto por IA
- O app encontra o assunto principal da foto no próprio computador (IS-Net): máscaras de assunto e fundo, efeitos aplicados só no assunto ou só no fundo, bokeh atrás do assunto.

### Visualização
- **Monitoramento**: histograma com avisos de recorte (**J**) e **vectorscope** com linha de tom de pele, antes / depois (**]**) e dividido (**Y**), tela cheia, zoom, fundo da área central escolhido com o botão direito.

### Várias imagens
- **Filmstrip** com várias fotos abertas, cada uma com os próprios layers; **sincronizar**, copiar e colar ajustes entre fotos.
- Formatos: JPG, PNG, HEIC, TIFF, RAW (CR3, ARW, NEF, DNG…).

### Exportar
- **JPG** (qualidade e limite de tamanho do arquivo) ou **PNG** (com transparência).
- Pasta específica, mesma pasta do original ou escolhida na hora, subpasta, o que fazer com arquivos que já existem.
- **Nome dos arquivos** montado com blocos (nome, texto, sequência, datas, dimensões), com modelos prontos.
- **Redimensionar** por largura e altura, lado maior, lado menor, megapixels ou porcentagem, em pixels, polegadas ou centímetros.
- Mostrar no Explorer / Finder ao terminar; **presets de exportação**.
- Copiar o resultado para a área de transferência.

### Projetos e app
- Projetos `.effector` (foto original + todos os layers editáveis); o app reabre a última sessão.
- Português e inglês, tema escuro e claro, todos os atalhos personalizáveis.
- Atualizações dentro do app (verifica a cada abertura e instala sozinho).

## Desenvolvimento

Rust + wgpu + egui. `cargo run --release` compila e abre o app. Notas técnicas em [`docs/DESENVOLVIMENTO.md`](docs/DESENVOLVIMENTO.md), guia de uso em [`docs/GUIA.md`](docs/GUIA.md) e notas de cada versão em [`docs/releases`](docs/releases).
