# Revisão ponta a ponta — Editor de Livros EPUB

Escopo lido: `apps/desktop` (React + Tauri/Rust), `packages/*` e o vault de decisões. Verificações feitas: `tsc --noEmit` e `vite build` passam; o bug do mouse foi reproduzido e a correção validada num Chromium real (veja abaixo). **O backend Rust não pôde ser compilado no ambiente de revisão** (faltam bibliotecas GTK do Linux): só a função `is_chat_model` foi compilada e testada isoladamente. Rode `cargo check` e `cargo test` no Windows antes de aceitar.

## Corrigido nesta rodada

1. **Remover chave de API.** Novo comando `delete_provider_api_key` e botão "Remover chave" com confirmação em duas etapas. Limpa também o catálogo e o modelo escolhido.
2. **Fluxo de adicionar chave e carregar modelos.** Antes eram três passos (salvar, testar, escolher). Agora é um botão: `connect_ai_provider` valida a chave **antes** de gravá-la (chave inválida não fica no cofre), carrega o catálogo e seleciona o modelo. Chave já salva com catálogo vazio carrega sozinha ao abrir. Enter envia. O catálogo exclui modelos que não são de chat (embeddings, áudio, moderação etc.), com fallback para a lista completa.
3. **Arrastar o mouse mudava de página.** Causa: as páginas são colunas CSS num contêiner com rolagem horizontal. Arrastar uma seleção até a borda dispara autoscroll do navegador, e o `onScroll` trocava a página. Reproduzido: o drag pulava para a página 27/27. Agora: o scroll é travado na página atual enquanto o botão está pressionado, a rolagem horizontal do trackpad é bloqueada, os botões laterais do mouse (voltar/avançar) são bloqueados e `overscroll-behavior: none` foi aplicado. A navegação fica só nas setas do app e no teclado: `Alt+←/→`, `PageUp/PageDown` e, na prévia somente leitura, `←/→`. Setas simples seguem movendo o cursor no modo edição.
4. `applySuggestion` usava `String.replace` com texto da IA como substituição: `$&` ou `$1` na sugestão corrompiam o XHTML. Agora usa função de substituição.

## Gaps encontrados (por prioridade)

**Alta**
- **Segurança do conteúdo.** `removeUnsafeMarkup` é uma sanitização por regex e é contornável. O conteúdo vai para `innerHTML` e `csp` está `null` no `tauri.conf.json`. Um XHTML malicioso pode executar JS com acesso aos comandos Tauri (`save_xhtml_document` grava em qualquer caminho .xhtml/.html; `analyze_open_document` usa a chave). Usar DOMParser com lista de permissão (ou DOMPurify), definir CSP e restringir `save_xhtml_document` à pasta do projeto aberto.
- **Perda silenciosa de dados.** A sanitização também remove `<form>`, `<script>`, `<object>` etc. do arquivo do autor ao salvar, sem aviso.
- **Backups poluem o EPUB.** `capítulo.xhtml.backup-<timestamp>` é criado ao lado do original; se a pasta for compactada em .epub, os backups entram no pacote. Mover para `.editor-backups/` fora do conteúdo, com limite de retenção.
- **O produto não abre nem gera EPUB.** Só abre pasta de XHTML. `packages/epub`, `validation`, `reader`, `backup` e `print-pdf` são apenas README. A ordem dos capítulos é por nome de arquivo e não pelo `spine` do OPF; imagens e CSS relativos não carregam no editor.

**Média**
- `save_xhtml_document` não é atômico (`fs::write` trunca o original). Gravar em temporário e renomear.
- `applySuggestion` procura o texto puro (`textContent`) no XHTML bruto: falha com entidades (`&amp;`, `&nbsp;`) e com trechos que cruzam tags. Também aplica na primeira ocorrência do capítulo, não na página analisada, e não escapa `after`.
- Desempenho: cada tecla recalcula a paginação e percorre os retângulos de todos os blocos (`composePageXhtml`). Em capítulos longos vai travar. Fazer debounce e calcular a página só ao analisar.
- `WorkspaceDashboard.tsx` mistura estado de IA, projeto, editor e UI em ~350 linhas com JSX em linhas únicas. Extrair `useAiSettings` e `useProject`.
- Limite de análise em bytes (`text.len()`), mas a mensagem fala em caracteres.
- Sem testes de frontend, ESLint ou CI. O lockfile foi gerado no Windows e falta o binário nativo do rolldown no Linux (`npm ci` + `vite` falham em CI Linux até instalar `@rolldown/binding-linux-x64-gnu`).

**Baixa**
- Versão divergente: `package.json` 0.1.0 vs `tauri.conf.json`/`Cargo.toml` 0.2.0.
- Só provedores com `/v1/models` compatível com OpenAI (NVIDIA, Groq, OpenAI). Faltam Anthropic, Gemini, OpenRouter, Ollama/LM Studio e URL base customizada.
- O texto do manuscrito vai entre `<manuscrito>` sem escapar `</manuscrito>` (injeção de prompt possível, impacto baixo porque a saída só gera sugestões revisáveis).

## Projetos parecidos consultados

A busca web retornou pouco para "editor EPUB com IA". Nada encontrado combina edição visual de EPUB com IA e Tauri.
- **Sigil** (GPLv3): referência para edição de EPUB (spine, OPF, navegação, validação). É o que mais cobre o gap de pacote EPUB.
- **calibre**: conversão e metadados; útil como referência de formatos, não de edição.
- **magicNovel** e **Bowri**: apps Tauri + React de escrita de romances com IA; propõem mudanças que o autor aprova antes de salvar, o mesmo princípio do seu fluxo de sugestões. Confira as licenças antes de reaproveitar código (magicNovel é PolyForm Noncommercial).
- **OpenWriter** (Tauri): editor com IA via Groq, exporta DOCX/PDF/HTML.

Não li o código desses projetos; a comparação vem só das descrições públicas.

---

# Rodada 2 — EPUB profissional, provedores e uso da pasta `skill`

## O que a pasta da skill realmente contém

- **Não existe um gerador de EPUB pronto.** `references/producao.md` descreve o pipeline (estrutura, CSS Kindle, template XHTML, regra do `mimetype`, checklist), mas o `assemble_epub.py` citado nele **não está no repositório**. O único código de EPUB é `regenerar_epub` em `tools/revisar_traducao_completa.py`, que só substitui capítulos dentro de um EPUB original e tem o nome do arquivo fixo (`Ascensao_de_Aria_FINAL.epub`). `INDEX.md` também cita `PROMPT_REVISAO_AUTOMATICA.md`, que não existe.
- **O que foi aproveitado:** a especificação de `producao.md` virou o gerador (`packages/epub`): estrutura, CSS base (`assets/stylesheet.css`), convenção `chapter-number`/`chapter-title` para o sumário, `mimetype` primeiro e sem compressão, e o checklist de validação (capítulo vazio/truncado, OPF × NCX, capa, UTF-8).
- **Não aproveitado, de propósito:** remoção de watermark/DRM de terceiros (`encryption.xml`) descrita em `producao.md`.
- **`corrigir_fluidez_ptbr.py`:** as 60 regras foram afinadas para uma tradução específica (ex.: "enganou as coxas", "bombeando-a cheia"). Como correção automática genérica, tem risco de falso positivo. Proposta: expor como **sugestões revisáveis** (antes/depois), nunca aplicar em lote.
- **Ainda subutilizado (próximo passo):** `SKILL.md` e `references/*` (perfis de revisão, Genesis Score, glossário, tabela de equivalências, pipeline). Hoje o app usa 5 prompts curtos em `lib.rs`; carregar os módulos relevantes por perfil seria o ganho editorial mais forte.

## Entregue

- **EPUB:** `packages/epub` (Rust, 18 testes) + botão **Exportar EPUB** (metadados, capa, validação com relatório, gravação atômica). Dois modos: pasta de capítulos (gera o pacote EPUB 3 completo) e EPUB extraído (reempacota sem tocar no conteúdo). Validado com o **epubcheck 5.2.1: 0 erros** nos dois modos. O epubcheck apontou um erro real no meu primeiro gerador (`nav` não linear sem link), já corrigido.
- **Bug encontrado:** o editor gravava `&nbsp;` no XHTML, entidade inválida em XML (leitores e epubcheck rejeitam). Corrigido na origem e também normalizado no empacotamento.
- **Provedores:** 10 servidores (NVIDIA, Groq, OpenAI, OpenRouter, Gemini, Mistral, DeepSeek, xAI, Together, Cerebras). O servidor é detectado pelo prefixo da chave colada; o seletor de modelos agrupa **por servidor**; modelos que não são de chat são filtrados; OpenRouter valida a chave em endpoint próprio (o catálogo dele é público). Modelos de raciocínio da OpenAI passaram a usar `max_completion_tokens`.

## Limites conhecidos

- Provedores novos seguem a documentação pública, mas **não foram testados com chaves reais**; a leitura do catálogo está coberta por testes com respostas de exemplo. Anthropic ficou de fora: usa protocolo próprio (`x-api-key`), não o formato OpenAI.
- Em modo `extracted`, EPUB 2 é reempacotado como EPUB 2 (com aviso); não há conversão automática para EPUB 3, nem troca de capa ou série.
- A ordem dos capítulos no modo "pasta" é a do painel (ordem natural dos nomes), não um `spine` editável.
- O Kindle Previewer/KDP não foram executados.
