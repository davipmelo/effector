# Notas de desenvolvimento (para o Claude)

## Base única em Rust (macOS + Windows) — desde a v0.6.0

- **Stack:** Rust + wgpu (Metal no Mac, DirectX 12/Vulkan no Windows) + egui/eframe para a interface.
- **Compilar e rodar:** `cargo run --release`. Abrir arquivos direto: `cargo run --release -- foto.jpg`.
- **Shaders:** os 63 kernels do app Swift foram convertidos de Metal para GLSL compute (`src/shaders/kernels/*.glsl`,
  com `header.glsl` + `prelude.glsl`). O naga compila tudo em tempo de execução; `cargo run --bin shadercheck` valida todos.
  Bindings: 0 = saída (rgba16f), 1 = floats `u`, 2 = floats `ex`, 3–6 = samplers (lin, near, rep, mirror), 7+ = texturas de entrada.
- **Arquitetura:** `model.rs` (LayerNode, Params, DocumentState — JSON compatível com o Codable do Swift) →
  `renderer.rs` (grafo com cache por hash) → `effects/*.rs` (um arquivo por grupo de efeitos, cada efeito é declarativo)
  → `gpu.rs` (contexto de render, pool de texturas por thread, blur/resize) → `ui/*.rs` (egui).
  `app.rs` é o equivalente do AppState (documentos, undo, sessão, projeto, export).
- **Novo efeito:** criar a função em `src/effects/<grupo>.rs` com `Effect::new(...)`, `.params(...)` e o `apply`,
  colocar o kernel em `src/shaders/kernels/k_<nome>.glsl`, registrar em `effects::all()` e traduzir os textos (`src/l10n_pt.rs`).
- **Fuzz:** `effector --fuzz <img> [iterações]` (parâmetros aleatórios/extremos em todos os efeitos, acusa pânicos).
- **Log de erros:** `effector.log` na pasta de suporte (erros de GPU, pânicos, reinícios da thread de render).
- **Efeitos ocultos:** `.hidden()` mantém um efeito só para projetos antigos (fora do navegador e da paleta).
- **Teste headless:** `effector --render-test <img> <outdir> [specs] [--check-neutral] [--full|--full2x]`
  (`spec`: `efeito:chave=valor,...`, `+` empilha, `presets` ou `presets=Categoria`); `--lint` verifica chaves, padrões,
  looks, presets e compila os shaders; `--dump-params` lista todos os parâmetros.
- **Teste da interface:** variáveis `EFFECTOR_SHOT=<png>` e `EFFECTOR_SHOT_SCRIPT="effect:vhs,select:0,browser,..."`
  abrem o app, executam os passos, salvam um screenshot da janela e fecham. Use `EFFECTOR_SUPPORT_DIR=<pasta>` para não
  mexer nas configurações / sessão reais.
- **Assunto por IA (0.9.0, substitui a profundidade da 0.8):** `src/subject.rs` roda o IS-Net general use (DIS,
  Apache-2.0, ONNX do projeto rembg) com **tract** (Rust puro, CPU, igual no Mac e no Windows) numa thread dedicada,
  ~2 s por foto em 1024×1024, só quando `subject::layers_need_subject` diz que algum layer ou máscara usa. Matte 0..1
  (1 = assunto) da imagem inteira; `Ctx::subject()` recorta e amplia com filtro bilateral guiado pela foto (`k_matteup`).
  O encanamento é o da antiga profundidade: `depthBias` "near"/"far" (chaves antigas) = assunto/fundo, o Background
  Blur usa o matte como "distância" (foco em 1), o Fog põe névoa onde o matte é 0. `.subject(|p| …)` declara uso.
  Pesos: `assets/models/subject_isnet.onnx` (fora do git, ~170 MB) baixado por `packaging/fetch_model.sh` (tenta o
  release `subject-model-v1` deste repo e depois o release do rembg, com SHA-256); vai em
  `EFFECTOR.app/Contents/Resources/models` e `{app}\models`. `effector --subject-view <img> <out.png>` testa.
- **Máscaras (0.9.0):** `MaskShape` = Linear, Radial, Brush, Subject, Background (+ `Depth` antigo, migrado para
  Subject/Background em `adopt_masks` via `Mask::migrate`). Posições em coordenadas da imagem inteira (0..1, sem
  recorte), tamanhos em frações do lado maior; `Ctx.crop` / `Ctx.aspect` (de `Extras`) mapeiam para o render.
  `renderer::part_texture` desenha cada parte (`k_maskgeom` para gradientes, `brush::raster` na CPU até 1280 px +
  upload para o pincel, `k_subjectmask` para assunto/fundo). O renderer guarda em cache a saída do efeito **sem** a
  máscara (`LayerNode::effect_hash`), então pintar ou arrastar um gradiente só refaz a mescla.
  UI: `ui/tools.rs` (painel, `MASK_KINDS`, `begin_kind`, cores dos modos em `mode_style`), `ui/mask_canvas.rs`
  (desenhar, pintar, `hit_test` por prioridade: bolinhas → alfinete → anel de rotação → elipse da suavidade / linhas
  do linear; `Op` guarda o arraste em andamento; ⇧ restringe todo arraste; ⌘/Alt + botão direito = tamanho e
  suavidade do pincel; `Geo` converte tela ↔ imagem), `AppState::{mask_draw, mask_draw_into, selected_part, brush}`,
  `AppState::mask_key` (teclas [ ] ⇧[ ⇧] ' ⌫ ⎋ antes dos atalhos globais). Atalhos: `MaskBrush` (K), `MaskLinear` (M),
  `MaskRadial` (⇧M), `MaskSubject` (⇧S), `MaskBackground` (⇧G). Specs do `--render-test`: `mask=linear|radial|brush|
  subject|background[/inv]`; passos do script de screenshot: `mask:<tipo>`, `draw:<brush|linear|radial>`.
- **Layers (0.9.0):** mesclagem + opacidade do(s) layer(s) selecionado(s) numa barra no topo da lista
  (`layers::layer_props_bar`) e o slot de máscara fixo em cada linha (`layer_mask_menu`); a seção "Layer" saiu do
  inspector e o Solo foi removido. ↩/⎋ voltam para Editar a partir de Máscaras e Canvas (`AppState::mask_key`).
- **Monitoramento (0.9.0):** `ui/histogram.rs` tem as abas Histograma e Vectorscope (`settings.monitor_tab`,
  `scope_zoom`). `Histogram::from_rgba` também acumula Cb/Cr Rec. 709 em `SCOPE`×`SCOPE` bins com a cor média
  (`scope`, `scope_rgb`); a imagem do scope vira uma textura egui guardada por medição. Linha de pele a 123°.
  Passo `scope` no script de screenshot.
- **Recorte personalizado (0.9.0):** `canvas_view::parse_ratio` ("2,39x1", "2.39:1", "16/9"), `crop_aspect` =
  `custom:<texto>`, recentes em `settings.crop_custom`.
- **Filtros de lente (0.9.0):** `lensfilters::diffusion` (difusão física em luz linear que conserva energia:
  `k_diffsrc` + 3 blurs + `k_diffusion`, constantes por filtro em `diffuser`) e `lensfilters::ghost` (`k_ghostfx`,
  cópias/rastro misturados em luz linear). Categoria de presets "Lens Filters".
- **Extras do render:** `renderer::Extras` leva o matte do assunto, o recorte, a proporção da imagem + data EXIF (`src/exif.rs`, DateTimeOriginal) para `Ctx`.
- **Atualizações (0.8):** `src/updater.rs` lê `releases/latest` do GitHub (repo público), baixa o asset da plataforma e instala
  (Windows: Inno Setup `/SILENT`, o `[Run] Check: WizardSilent` reabre o app; Mac: script que espera o app fechar, troca o
  `.app` pelo do DMG e reabre). Testes: `--check-update [--install]` com `EFFECTOR_FAKE_VERSION=0.6.0` e
  `EFFECTOR_UPDATE_NO_REOPEN=1` num `.app` falso.
- **Looks e paletas (0.8):** looks extras em `effects/looks080.rs`; combinações de cores por efeito e grupos novos de
  degradês/paletas em `effects/palettes080.rs` (o inspector mostra o seletor "Paleta" na seção da primeira cor).
- **Ferramentas de dev:** `--list-looks`, `--slider-audit`, `--missing-pt` (textos sem tradução), `look=<id>` e
  `mask=<tipo>` nas specs do `--render-test`, passos `mask:` e `draw:` no script de screenshot.
- **CI:** `.github/workflows/ci.yml` compila e roda o lint no Windows e no Mac a cada push na `main`.
- **Traduções 0.8:** `src/l10n_pt_080.rs` (sobrepõe `PT`); 0.8.1 em `src/l10n_pt_081.rs`; 0.8.2 em `src/l10n_pt_082.rs`;
  0.9.0 em `src/l10n_pt_090.rs`.
  Cuidado com chaves curtas ("Add", "Custom Text"…): uma tabela nova sobrepõe a tradução antiga em todo o app.
- **Histograma (0.8.1):** `src/histogram.rs` mede a saída na thread de render (cópia de ≤640 px, só com o painel aberto);
  `ui/histogram.rs` desenha. **J** = `Action::ShowClipping` (cópia de exibição via `k_clipview`), ⇧⌘H abre/fecha.
- **Paridade preview/export (0.8.1):** `Ctx::content_key` (imagem + recorte, `worker::content_key`) e `Ctx::layer_key`
  (cadeia de hashes sem a resolução) são iguais no preview e no export. Efeitos que analisam a foto (Blob Tracker) guardam
  o resultado do preview por `layer_key`, e o export reaproveita. `--render-test <img> <dir> <specs> --parity` mede a diferença.
- **Fontes (0.8.1):** parâmetro padrão `overlay::font_def(seção)` + `overlay::font_param(p)` → `font_name` `"fx:<id>"`.
  Fontes extras em `overlay::EXTRA` (OFL, `assets/fonts/OFL-*.txt`).
- **Ferramentas (0.8.2):** `app::Tool` (Edit, Crop, Canvas, Masks) + `ui/tools.rs` (fileira de ícones, painéis de Crop,
  Canvas e Masks, linha "Máscara" do layer). `set_tool` aplica o recorte ao sair de Crop. Atalhos `ToolEdit` (E),
  `ToolCanvas` (⇧B), `ToolMasks` (⇧W), `Crop` (R). (O painel de máscaras foi refeito na 0.9.0, ver abaixo.)
- **Máscaras (0.8.2):** `DocumentState.masks: Vec<Mask>` (partes `MaskPart` com `MaskMode` add/subtract/intersect e
  `MaskShape::Depth`); o layer aponta com `LayerNode.mask: Option<MaskRef>`. Para renderizar use sempre
  `DocumentState::render_layers()` (preenche `resolved_mask`); `renderer::mask_texture` desenha a máscara
  (`k_depthmask` + `k_maskop`). `adopt_masks` (chamado em todo `edit` e ao ler um `DocumentState`) converte
  `depth_mask` antigo e `embedded_mask` (cópia que viaja em presets, clipboard e sync, via `portable`) em máscaras
  da imagem. Sobreposição vermelha: `RenderRequest.mask_overlay` → `k_maskview` → `ui/tools::paint_mask_overlay`.
- **Export (0.8.2):** `src/export.rs` (`ExportSettings`, modelo de nome `NameToken`, `plan` / `run`, limite de tamanho
  do JPG por busca binária na qualidade, `reveal`) + `ui/export_dialog.rs`. As configurações e os presets de export
  ficam em `settings.json` (`export`, `exportPresets`).
- **Zoom e padrões finos (0.8.2):** efeitos com `.fine_pattern()` (Dither, Halftone, ASCII) renderizam o preview na
  escala 1; o worker reduz a cópia de exibição para `RenderRequest.display_size` (tamanho na tela) com Lanczos.
  Mudança isolada no commit "Zoomed-out preview without false patterns" (fácil de reverter).
- **Atualização ao abrir (0.8.2):** verifica a cada abertura; `AppState.show_update_popup` → `dialogs::update_popup`
  (respeita `settings.skipped_version`). Passo `update` no script de screenshot simula uma versão nova.
- **Passos novos do script de screenshot:** `masks`, `canvas`, `edit`, `hsladj`, `update`; `dm:a/b` agora cria uma
  máscara da imagem no layer selecionado.
- **Tab / setas:** `raw_input_hook` tira o Tab da entrada do egui (senão ele move o foco); pickers usam `set_focus_lock_filter`
  e consomem as setas enquanto estão abertos.
- **Largura das sidebars:** uma linha mais larga que o painel faz o egui alargar a sidebar a cada frame. Calcule larguras
  com `item_spacing` e nunca passe de `available_width()`.
- **Unidades de tamanho:** relativas a 1000 px no lado maior (`renderer::REFERENCE_SIZE`).
- **Regra P&B:** saída com R=G=B exato; o renderer restaura o alfa de entrada (`k_withalpha`).
- **Dados do usuário:** pasta `EFFECTOR` em Application Support (Mac) / `%APPDATA%` (Windows): `settings.json`,
  `presets.json`, `swatches.json`, `LUTs/`, `Session/`. Os formatos de presets e paletas são os mesmos do app Swift.
- **README (obrigatório):** o `README.md` resume todas as funções do app. Toda vez que uma função, efeito, ferramenta
  ou preset for adicionado ou removido, atualizar o README (contagens de efeitos e presets incluídas) no mesmo commit.
- **Patch notes (obrigatório):** antes de cada release, escrever `docs/releases/vX.Y.Z.md` em PT-BR explicando tudo
  o que mudou (correções, efeitos, presets, bastidores) + instruções de instalação. O workflow publica esse arquivo como
  texto do release. Modelo: `docs/releases/v0.7.0.md`.
- **Release:** criar a tag `vX.Y.Z` e dar push → `.github/workflows/release.yml` compila no Windows (instalador Inno Setup)
  e no Mac (`.app` universal + `.dmg`) e publica o release. Versão em `Cargo.toml`.
- **Mac local:** `packaging/macos/build_app.sh [--install]`.
- **Ícone:** desenhado em código (`src/appicon.rs`); `effector --write-icons assets/icons` regera `.ico`, `.icns` e `.png`.

## App Swift antigo (arquivado)

A versão Swift original (só macOS) foi arquivada depois que a versão Rust foi validada no Mac.
Ela não está mais na `main`: fica na branch `swift-legacy` e na tag `swift-final`
(`git checkout swift-final` para consultar). `tools/` guarda os scripts usados na conversão Swift → Rust;
eles leem `Sources/`, então só funcionam a partir desse checkout antigo.
