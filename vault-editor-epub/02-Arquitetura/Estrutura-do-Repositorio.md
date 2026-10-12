# Estrutura do repositório modular

## Decisão

O produto será um monorepo modular. O app desktop é apenas a composição dos módulos; regras de EPUB, XHTML, PDF, IA e domínio não ficam espalhadas na interface. Esta árvore é o ponto de partida para a criação do código — não moveremos o protótipo atual até iniciar a migração.

```text
Editor de Livros EPUB/
├─ apps/
│  └─ desktop/                    # shell Tauri/Electron, janelas e permissões
│     ├─ src/                     # composição da UI e rotas
│     └─ src-tauri/               # host nativo, somente adaptadores e comandos
├─ packages/
│  ├─ domain/                     # entidades, invariantes, casos de uso e contratos
│  ├─ project/                    # índice de projeto, histórico e configuração local
│  ├─ xhtml/                      # parse, serialização, schema, transformações e sanitização
│  ├─ epub/                       # container.xml, OPF, manifest, spine, nav, import/export ZIP
│  ├─ validation/                 # regras XHTML/EPUB, adaptador EPUBCheck e diagnósticos
│  ├─ editor/                     # editor fonte/visual, transações, undo/redo e diff
│  ├─ reader/                     # iframe sandboxed, paginação reflow e presets Kindle
│  ├─ backup/                     # snapshots, hashes, retenção e restauração
│  ├─ editorial-rules/             # skill editorial em perfis, checks e contratos de saída
│  ├─ ai/                         # prompts versionados, contexto, provedores e sugestões
│  ├─ print-pdf/                  # composição paginada, perfis de impressão e exportação
│  ├─ ui/                         # design system sem regras de negócio
│  └─ platform-contracts/         # portas FileStore/SecretStore/ProcessRunner etc.
├─ tooling/
│  ├─ epubcheck/                  # integração empacotada ou instruções de instalação
│  └─ scripts/                    # automação de build, nunca lógica editorial de produção
├─ test-fixtures/
│  ├─ xhtml/                      # capítulos válidos, inválidos e casos de preservação
│  ├─ epub/                       # EPUB2/EPUB3, links, imagens, nav e casos maliciosos
│  └─ golden/                     # PDFs/EPUBs de referência para regressão
├─ vault-editor-epub/             # este vault Obsidian: produto, ADRs e decisões
├─ editor-profissional-de-livros_skill/ # fonte da expertise editorial existente
└─ editor_xhtml_profissional_pagina_o.html # protótipo preservado durante a migração
```

## Regras de dependência

```text
ui / editor / reader / desktop
              ↓
   domain + platform-contracts
              ↑
xhtml / epub / validation / backup / ai / print-pdf / project
              ↓
     adaptadores nativos, CLIs e provedores externos
```

- `domain` não depende de React, Tauri, Electron, ZIP, banco ou rede.
- `xhtml` e `epub` não dependem de componentes visuais.
- `reader` nunca grava no documento; `editor` nunca chama filesystem diretamente.
- `ai` retorna achados e operações propostas, não muta `Document` sozinho.
- `print-pdf` lê a publicação normalizada e gera uma saída separada; não altera o EPUB.
- Integrações externas ficam atrás de adaptadores implementados no desktop/tooling.

## Responsabilidade do app desktop

Somente: inicialização, seleção de arquivos, permissões, ponte segura para o sistema, ciclo de atualização e composição da UI. Se houver uma regra que também faria sentido num CLI ou num futuro app web, ela não pertence a `apps/desktop`.

## Convenções iniciais

- TypeScript estrito nas packages compartilhadas; Rust apenas em adaptadores nativos que se beneficiem de acesso ao SO ou streaming de arquivos.
- Testes unitários ao lado do módulo; fixtures EPUB/XHTML centralizados para testes de integração.
- Contratos públicos de packages em `src/index.ts`; imports internos profundos são proibidos.
- Cada package é dona de seu `README`, testes e changelog de alterações incompatíveis.

