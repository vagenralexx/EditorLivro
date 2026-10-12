# Bibliotecas, exportação PDF vetorial e spikes

## Regra de seleção

Bibliotecas são candidatas, não decisões irreversíveis. Cada uma só entra após um spike com EPUBs reais, arquivos com português/acentos, imagens, CSS e links internos. O contrato do módulo protege o restante do produto de troca futura.

## Conjunto recomendado para avaliar

| Área | Candidato principal | Papel | Decisão/alerta |
|---|---|---|---|
| Shell desktop | Tauri 2 | janela nativa, permissões e comandos locais | primeira opção; usar permissões por escopo, não acesso amplo ao disco |
| Arquivos nativos | `@tauri-apps/plugin-fs` + adaptador Rust | leitura/escrita com escopo e operações atômicas | o frontend só acessa caminhos autorizados; a própria documentação exige permissões e scopes explícitos |
| Diálogos | plugin de diálogo do shell | escolher pasta/EPUB/destino de exportação | encapsulado em `FileStore` |
| Persistência | SQLite via adaptador nativo | índice, sessões, histórico, relatórios e preferências | não guardar conteúdo ou chaves em claro sem necessidade |
| Segredos | keychain/secure store do SO | chaves Groq/OpenAI e tokens | não usar `localStorage`, `.env` distribuído ou banco comum para segredos |
| ZIP/EPUB | crate Rust `zip` + `quick-xml` (ou equivalentes atrás de `ArchiveStore`) | extração segura, container/OPF/nav e reempacotamento | não usar `epub.js` como núcleo de escrita; ele é candidato à renderização |
| Render de EPUB | `epub.js` | prévia/leitura de EPUB no browser | avaliar para leitor, não para preservar/editar o pacote fonte |
| XML/XHTML | parser XML estrito no núcleo; `@xmldom/xmldom` ou `saxes` apenas se o adapter TS se provar necessário | validar e transformar sem tratar XHTML como HTML solto | a serialização deve preservar namespaces, entidades e atributos relevantes |
| Editor fonte | CodeMirror 6 | edição de XHTML, atalhos, realce e diagnósticos | é a superfície de edição canônica no MVP |
| Editor visual | ProseMirror com schema XHTML restrito | transações, undo/redo e nós permitidos | não entrar antes do schema/preservação passarem nos fixtures |
| Diff e histórico | operações do editor + diff textual/estrutural | revisão manual/IA e restauração | `ChangeSet` é independente da biblioteca visual |
| EPUBCheck | EPUBCheck oficial, executado como processo controlado | validação de conformidade de EPUB2/EPUB3 | referência final; incluir custo de distribuição/JVM no spike |
| Paginação para PDF | Vivliostyle CLI | compor HTML/EPUB em PDF tipográfico de alta qualidade | primeira opção para livro impresso e perfis CSS paginados |
| Prévia de impressão | Paged.js | paginação no browser e testes de CSS Paged Media | útil para prévia; não substitui o renderizador de exportação até comparado no corpus |
| Pós-processamento PDF | `pdf-lib` | metadados, bookmarks, anexos, merge e elementos vetoriais auxiliares | **não** é motor de diagramação de romance; usar depois da composição |

## PDF vetorial: definição e caminho

O PDF de livro deve preservar texto e formas/SVG como vetores; imagens inseridas continuarão raster, na resolução original. Não vamos converter páginas em screenshots/canvas para exportar, pois isso destrói seleção de texto, busca, acessibilidade e nitidez.

### Pipeline proposto

```text
EPUB/XHTML + perfil de impressão + CSS paginado
        → normalização para entrada de impressão
        → Vivliostyle CLI (composição HTML/CSS)
        → PDF vetorial
        → pós-processamento opcional (metadados, bookmarks, capa)
        → pré-flight e comparação visual
```

Vivliostyle é um candidato forte porque sua CLI aceita EPUB/HTML e gera PDF; Paged.js permanece valioso para visualizar paginação e validar regras no app. `pdf-lib` serve ao acabamento do PDF, não à composição reflow de um livro inteiro.

## Spikes obrigatórios

### S1 — núcleo EPUB seguro

- Importar EPUB2 e EPUB3 com capa, imagens, CSS, nota, nav e NCX.
- Alterar um parágrafo, exportar e validar com EPUBCheck.
- Reabrir o resultado em pelo menos um leitor e comparar manifest/spine/recursos.
- Exercitar paths maliciosos e ZIP muito grande; nenhuma escrita pode sair do workspace.

### S2 — fidelidade XHTML e editor

- Abrir fixtures com namespaces, `<em>`, `<a>`, imagens, entidades, atributos `epub:type` e CSS.
- Editar no fonte e no visual; confirmar que a alteração não remove marcação desconhecida.
- Demonstrar undo/redo e um diff estrutural estável.

### S3 — PDF de livro vetorial

- Gerar miolo A5 e 15×23 cm com cabeçalho, numeração, quebras de capítulo, notas, itálico e imagem.
- Verificar que o PDF permite selecionar/copiar texto e que SVG/texto não foram rasterizados.
- Comparar Vivliostyle e Paged.js no mesmo fixture; registrar diferenças de paginação e suporte CSS.

### S4 — segurança de dados e IA

- Salvar/recuperar uma chave no cofre do sistema sem expô-la à UI.
- Enviar apenas seleção consentida a um provedor de teste e aplicar resposta como diff reversível.
- Validar cancelamento, timeout, erro de provedor e ausência de rede.

## Fontes primárias consultadas em 2026-10-11

- A documentação do [Tauri File System](https://v2.tauri.app/plugin/file-system/) descreve acesso por permissões e scopes explícitos, e bloqueia traversal de caminho; isso reforça o adaptador de filesystem em vez de expor acesso irrestrito à UI.
- O [EPUBCheck](https://github.com/w3c/epubcheck/blob/main/README.md) é o verificador oficial de conformidade EPUB e pode ser chamado por linha de comando ou como biblioteca Java.
- [epub.js](https://github.com/futurepress/epub.js/) se apresenta como biblioteca de renderização de EPUB em browser, portanto é adequado à prévia, não como motor de preservação/escrita do arquivo-fonte.
- O guia do [ProseMirror](https://prosemirror.net/docs/guide/) enfatiza schema e transações, que são exatamente os requisitos para um editor visual controlado.
- A [Vivliostyle CLI](https://docs.vivliostyle.org/publications/cli-en/en/cli/getting-started/) gera PDF a partir de HTML e também aceita EPUB; [Paged.js](https://pagedjs.org/en/documentation/) é candidato de paginação baseada em CSS no browser; [pdf-lib](https://pdf-lib.js.org/) cria/modifica PDFs e desenha texto, imagens e gráficos vetoriais.

