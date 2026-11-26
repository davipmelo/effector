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
- **Profundidade (0.8):** `src/depth.rs` roda o Depth Anything V2 Small (Apache-2.0) com candle **no CPU** (o Metal do
  candle não tem layer-norm) numa thread dedicada (o modelo não é `Send`), ~0,9 s por foto em 518×518, só quando
  `depth::layers_need_depth` diz que algum layer usa. Mapa normalizado 0..1 (1 = perto) da imagem inteira; `Ctx::depth()`
  recorta e amplia para o tamanho do render com filtro bilateral guiado pela foto (`k_depthup`). `LayerNode.depth_mask`
  e o parâmetro padrão `depthBias` (via `.depth_bias()`) viram máscara em `renderer::layer_mask` (`k_depthmask`, `k_blendm`).
  Efeito que lê profundidade declara `.depth(|p| …)`. Pesos: `assets/models/depth_anything_v2_vits_f16.safetensors`
  (fora do git) baixado por `packaging/fetch_model.sh` do release `depth-model-v1`; vai em `EFFECTOR.app/Contents/Resources/models`
  e `{app}\models` no Windows. `effector --depth-view <img> <out.png>` testa.
- **Extras do render:** `renderer::Extras` leva profundidade + data EXIF (`src/exif.rs`, DateTimeOriginal) para `Ctx`.
- **Atualizações (0.8):** `src/updater.rs` lê `releases/latest` do GitHub (repo público), baixa o asset da plataforma e instala
  (Windows: Inno Setup `/SILENT`, o `[Run] Check: WizardSilent` reabre o app; Mac: script que espera o app fechar, troca o
  `.app` pelo do DMG e reabre). Testes: `--check-update [--install]` com `EFFECTOR_FAKE_VERSION=0.6.0` e
  `EFFECTOR_UPDATE_NO_REOPEN=1` num `.app` falso.
- **Looks e paletas (0.8):** looks extras em `effects/looks080.rs`; combinações de cores por efeito e grupos novos de
  degradês/paletas em `effects/palettes080.rs` (o inspector mostra o seletor "Paleta" na seção da primeira cor).
- **Ferramentas de dev:** `--list-looks`, `--slider-audit`, `--missing-pt` (textos sem tradução), `look=<id>` e
  `dm=from/to/feather[/inv]` nas specs do `--render-test`, passos `dm:` e `depth` no script de screenshot.
- **CI:** `.github/workflows/ci.yml` compila e roda o lint no Windows e no Mac a cada push na `main`.
- **Traduções 0.8:** `src/l10n_pt_080.rs` (sobrepõe `PT`); 0.8.1 em `src/l10n_pt_081.rs`; 0.8.2 em `src/l10n_pt_082.rs`.
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
  `ToolCanvas` (⇧B), `ToolMasks` (⇧W), `Crop` (R).
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
