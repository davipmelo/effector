# Notas de desenvolvimento (para o Claude)

- Build/instalar: `scripts/build_app.sh --install` (release + bundle + ícone + assinatura ad-hoc → /Applications/EFFECTOR.app).
- `build.env` define DEVELOPER_DIR/SDKROOT (o SDK do CommandLineTools desta máquina está quebrado).
- Shaders Metal compilados em runtime a partir de strings Swift (sem Metal Toolchain instalado).
- Teste headless: `.build/release/Effector --render-test <img> <outdir> [specs] [--check-neutral] [--full|--full2x] [--project-roundtrip]`
  - spec: `efeito:chave=valor,...` e `+` para empilhar (o primeiro fica embaixo); `presets` renderiza todos os presets de fábrica.
  - gera `_sheet.jpg` (folha de contato).
- Unidades de tamanho: relativas a 1000 px no lado maior (`Renderer.referenceSize`).
- Regra P&B: saída com R=G=B exato; o renderer restaura o alfa de entrada (`k_withalpha`) para não vazar cor.
- Arquitetura: Model (LayerNode, Params, DocumentState) → Renderer (grafo com cache por hash) → Effects (protocolo `Effect`, registro em `EffectRegistry`) → UI SwiftUI.
- Novo efeito: criar struct `Effect` com `params` e `apply`, adicionar o shader numa enum `*Shaders`, incluir no `ShaderLibrary.fullSource` e no `EffectRegistry.all`, e traduzir os textos em `PT.swift`.
