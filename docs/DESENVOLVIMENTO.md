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
- **Teste headless:** `effector --render-test <img> <outdir> [specs] [--check-neutral] [--full|--full2x]`
  (`spec`: `efeito:chave=valor,...`, `+` empilha, `presets` ou `presets=Categoria`); `--lint` verifica chaves, padrões,
  looks, presets e compila os shaders; `--dump-params` lista todos os parâmetros.
- **Teste da interface:** variáveis `EFFECTOR_SHOT=<png>` e `EFFECTOR_SHOT_SCRIPT="effect:vhs,select:0,browser,..."`
  abrem o app, executam os passos, salvam um screenshot da janela e fecham. Use `EFFECTOR_SUPPORT_DIR=<pasta>` para não
  mexer nas configurações / sessão reais.
- **Unidades de tamanho:** relativas a 1000 px no lado maior (`renderer::REFERENCE_SIZE`).
- **Regra P&B:** saída com R=G=B exato; o renderer restaura o alfa de entrada (`k_withalpha`).
- **Dados do usuário:** pasta `EFFECTOR` em Application Support (Mac) / `%APPDATA%` (Windows): `settings.json`,
  `presets.json`, `swatches.json`, `LUTs/`, `Session/`. Os formatos de presets e paletas são os mesmos do app Swift.
- **Release:** criar a tag `vX.Y.Z` e dar push → `.github/workflows/release.yml` compila no Windows (instalador Inno Setup)
  e no Mac (`.app` universal + `.dmg`) e publica o release. Versão em `Cargo.toml`.
- **Mac local:** `packaging/macos/build_app.sh [--install]`.
- **Ícone:** desenhado em código (`src/appicon.rs`); `effector --write-icons assets/icons` regera `.ico`, `.icns` e `.png`.

## App Swift antigo (só macOS)

O código Swift original continua em `Sources/` + `Package.swift` como referência até a versão Rust ser validada no Mac.
Build antigo: `scripts/build_app.sh --install`. `tools/` guarda os scripts usados na conversão Swift → Rust.
