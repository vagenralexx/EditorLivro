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

## Onde ficam os dados (round 5)

Nada importante fica no navegador/WebView. A pasta do projeto é a fonte da verdade e basta abri-la de novo após reinstalar ou formatar:

| Arquivo | Conteúdo | Observações |
|---|---|---|
| `.livro/projeto.json` | configurações, posição de leitura, caminho do original, metadados do EPUB, pasta do caderno e mapeamento | gravado por seção, atômico (fsync + rename), `.bak`; arquivo ilegível **nunca** é sobrescrito |
| `glossario.json` | glossário do projeto | formato anterior mantido |
| caderno (padrão `.livro-notas/`, ou **qualquer pasta**, ex.: o vault Obsidian da trilogia) | notas `.md`: fichas, fios, recaps, bíblias, registros | notas existentes nunca são reescritas só por serem lidas |
| `.livro/snapshots/*.zip` | cópias automáticas (≤1 a cada 5 min, 30 mais recentes) antes de renomear/excluir/aplicar recap/editar bíblia | restaurar guarda o estado atual como outra cópia |
| `.livro-exemplares.json` | exemplares com marca d'água | formato anterior mantido |

Dados antigos do `localStorage` são migrados uma vez (e só apagados depois de gravados). **Evitamos SQLite** como fonte da verdade: bancos corrompem em pastas sincronizadas (OneDrive/Dropbox) e não são legíveis/editáveis à mão.

**Registros por ID** (`CF-20`, `FIO-14`, `LAC-03`, `PT-L1-01`…): linhas de tabela Markdown são lidas como registros (aba *Registros*), com o estado vigente **inferido** do texto (nunca gravado nos arquivos). Verificado nos arquivos reais do autor: 52 IDs CF / 54 linhas, idêntico a uma contagem independente.

## Bíblia em níveis (série › livro › arco)

- Cada bíblia é uma nota `.md` (`nivel:` e `livro:` no frontmatter) na pasta de bíblias do caderno; seções `## N. Título`. Uma bíblia existente do autor é adotada como está (verificado na *Biblia-L1-A-Cura*: 14 seções reconhecidas; regravar todas as seções sem mudar nada preserva o conteúdo).
- O app grava **uma seção por vez**; o resto do arquivo fica intacto. Seção endereçada por título exato; se houver duas do mesmo tipo (como "Mecânica…" e "Regras do universo") nunca se escolhe por palpite.
- Estado de canon na última linha da seção: `#canon/proposto` (veio da IA), `#canon/confirmado`, `#canon/aberto` (lacuna). A memória enviada à IA rotula proposto/lacuna e esconde as tags.
- Preenchimento: (1) manual, guiado por *para que serve / formato / perguntas* de cada seção; (2) IA **a partir das suas respostas** (só estrutura, não inventa); (3) IA **a partir do manuscrito**, em blocos de ~40 mil caracteres acumulando o rascunho. A IA só propõe; você revisa/edita e aplica.
- Ainda não feito: importar bíblia de `.docx`, séries com vários livros no mesmo vault com herança entre níveis na memória, e "bíblia compilada" como visão gerada.

## Substituir no livro (round 6)

"Substituir no livro…" (barra lateral): troca **palavras inteiras** no capítulo aberto ou no livro todo.

- Palavra inteira: "Caio" não mexe em "Caiozinho"/"Caios" (elas são contadas e listadas como "parecidas, ignoradas"). A caixa acompanha o original (Caio→Jordan, caio→jordan, CAIO→JORDAN); acentos exatos por padrão (opção para ignorar).
- Só **texto**: em XHTML nunca toca tags, atributos (`id`, `class`, `href`), comentários, `<script>`/`<style>` ou entidades. Em Markdown preserva alvos de `[[links]]`, código e URLs; `[[Caio Barbosa]]` vira `[[Caio Barbosa|Jordan Barbosa]]` (o link continua apontando para a mesma ficha).
- Desempenho: o livro inteiro é rápido (texto puro; a análise lê os arquivos e conta, sem gravar). O risco de "livro todo" não é peso e sim erro/irreversibilidade, por isso: prévia por capítulo com amostras, seleção capítulo a capítulo, confirmação, backup e desfazer.
- Impacto na história: lista fichas, bíblia, registros, recaps e glossário que citam o termo, cada um com contagem e caixa de seleção; a ficha do nome recebe a opção de ganhar o nome novo como **apelido** (senão a memória deixaria de reconhecê-la nos capítulos).
- Segurança: originais copiados para `.livro/substituicoes/<id>/` + manifesto com impressão digital; falha no meio reverte o que foi gravado; **desfazer** só restaura arquivos não editados depois (ou à força, avisando que perde edições posteriores). Capítulo aberto é salvo antes e recarregado depois. Arquivos de ficha **não são renomeados** (links e a convenção `Nome-Sobrenome.md` do autor ficam intactos).
- Verificado nos arquivos reais: Caio→Jordan acharia 14 ocorrências na bíblia, 16 no registro de conflitos, 3 na cronologia etc., sem tocar links.

Aprendido com as fichas/índices enviados: `Indice.md`, `Matriz-*` e `Mapa-*` dentro da pasta de personagens deixam de ser tratados como personagens (antes "índice" no texto puxaria a nota para a memória); a lista mostra o título `# Eugênio Castelo` em vez do nome do arquivo; o modelo de ficha nova segue o padrão do autor (Ficha técnica Campo|Registro + Vida própria R10 + Aparições + Arco e conexões).

## Referências no editor, renomear ficha e fluxo de trabalho (round 7)

- **Referências** (botão no topo): lista os personagens/lugares/fios do caderno citados na **página** ou no **capítulo** abertos (reconhece título, apelidos e primeiro nome; o apelido novo após uma troca de nome também conta). Um clique abre o dossiê: **Ficha**, **Relações** (links de saída e backlinks), **Registros** (CF/FIO/LAC/PT que citam o nome, com estado inferido), **Bíblia e notas** (seções que citam, com o estado de canon, e outras notas). Os links do dossiê navegam dentro do painel; há atalhos para abrir a nota no Caderno e para "Substituir nome…" já preenchido.
- **Renomear a ficha** na substituição: o arquivo segue o padrão do autor (`Caio-Barbosa.md` → `Jordan-Barbosa.md`, sem acento se o antigo não tinha), os `[[links]]` de todas as notas (por título ou caminho) são reescritos, e o desfazer devolve arquivo e links. Se a ficha é renomeada, o nome novo já é o título e não precisa de apelido.
- **Salvar capítulo**: backups saíram de ao lado do capítulo (`.backup-<ts>`, sem limite) para `.livro/backups/` (10 mais recentes por arquivo); gravação atômica; o comando agora exige a pasta do projeto e recusa arquivos de fora dela.
- **Fechar a janela com alterações não salvas** pergunta: salvar e sair / sair sem salvar / continuar editando.
- **Botão "Analisar" fixo** no rodapé do painel do assistente (ficava abaixo da dobra em telas de 768–900 px).

## Revisão de UI/UX e fluxo de trabalho (round 8)

Auditoria com **axe-core** (WCAG 2.1 A/AA + boas práticas) em cada tela e diálogo, em modo claro e noturno, mais teste de teclado. De **4 tipos de violação (≈100 elementos)** em várias telas para **0**.

- **Contraste:** ~40 cores de texto cinza-claro (3:1) viraram AA (≥4.5:1); modo noturno recebeu sobrescritas próprias (painéis, Caderno e Bíblia). Textos de 8–9 px subiram para 10 px.
- **Diálogos** (`shared/useDialogA11y.ts`, usado em todos): foco inicial dentro, **Tab preso** no diálogo (grupo de rádio = uma parada), **Esc** fecha (o mesmo manipulador do ×, com confirmação quando há texto não salvo), foco **volta** ao botão que abriu; diálogos aninhados (recap dentro do Caderno) respondem só o do topo.
- **Estrutura:** `h1` na marca, níveis de título sem saltos, um único `main`, `aside` com nome, SVG do grafo sem controles aninhados, diálogo de configurações deixou de ser `form role=dialog`.
- **Foco visível** global e `prefers-reduced-motion`.
- **Robustez:** `ErrorBoundary` na raiz (tela explicativa em vez de tela branca; nenhum arquivo é tocado).
- **Ferramentas do livro** (Referências, Caderno, Bíblia, Substituir, Glossário, Exportar) agrupadas na barra lateral esquerda; a barra superior ficou só com Configurações e Abrir projeto; a Bíblia abre direto dali.
- **Fluxo:** atalhos **Alt+R** (referências), **Alt+H** (substituir no livro), **Alt+N** (caderno); painel de referências abre abaixo da barra superior, sem esconder os botões; registros por nome (Mapa de Âncoras, Matriz de Relações, Verificação de Pontas Soltas, 42 itens nos arquivos do autor) na aba Registros.
- **Janela:** largura mínima do app é 1080 px (desktop), então 320/768 px da checklist não se aplicam; verificado em 1080, 1366 e 1440 px, sem rolagem horizontal.

## Esforço de raciocínio por provedor (round 9)

| Provedor / modelo | Níveis oferecidos | Padrão | Fonte da informação |
|---|---|---|---|
| NVIDIA `z-ai/glm-*` | baixo, alto, máximo | baixo | verificado ao vivo (início do projeto) |
| NVIDIA Nemotron 3 Super/Ultra | liga/desliga pensamento (sem níveis) | desligado | documentação do modelo |
| Groq `gpt-oss-20b/120b` | baixo, médio, alto | baixo | **referência oficial da API Groq** (colada pelo autor); chamada real ainda não testada |
| Groq `qwen3-*` | sem raciocínio (none), padrão (default) | none | referência oficial da API Groq |
| Groq `qwen3.8-*` | sem raciocínio, baixo, médio, alto | none | referência oficial da API Groq |
| DeepSeek (qualquer modelo) | sem raciocínio, alto, máximo (`thinking` + `reasoning_effort`) | sem raciocínio | guia "Thinking Mode" (via buscador; **não verificado ao vivo**) |

Rede de segurança: se o servidor responder 400/422 citando `reasoning`, `reasoning_effort` ou `thinking`, o app repete a chamada **uma vez** sem esses parâmetros (como já fazia com o modo JSON). Outros modelos da Groq (ex.: Llama) e demais provedores continuam sem parâmetro de esforço.

## Primeiro teste no Windows (round 10)

Achados do teste real e correções:

- **Prévia Kindle → Editar deixava o texto cortado nas duas bordas.** Causa: a largura da coluna muda entre os modos, mas o scroll horizontal ficava no mesmo pixel (meio de uma página). Agora a página atual é guardada e o scroll é realinhado ao novo passo; verificado em 1100, 1366, 1440 e 1920 px, indo e voltando.
- **Salvar no Caderno:** botão "Salvar" visível, estado "✓ salvo / alterações ainda não salvas" e **Ctrl+S**; o salvamento automático (1 s após parar de digitar) continua.
- **Lista do Caderno:** só o grupo da nota aberta fica expandido (ou os que têm resultado da busca); os demais começam recolhidos.
- **Registros:** com o vault real eram **1618** itens misturando IDs e linhas de tabelas. Agora há duas visões: *Com ID* e *Tabelas por nome*, com filtros próprios e limite de 300 itens na lista (use busca/filtros).
- **Índices não são bíblias/personagens/registros:** `Indice*`, `README`, `MOC`, `Sumário` viram nota comum em qualquer pasta (o `01-Biblias/Indice.md` aparecia como bíblia); `Matriz*`/`Mapa*`/`Quadro*` em pastas de personagens/bíblias viram Registro.
- **Bíblia:** "Seções que faltam" fica recolhida (abre sozinha ao escolher uma delas).
- **Tema escuro de ponta a ponta:** diálogos (configurações, exportar, glossário, substituir, dados do projeto), painel de referências, Caderno e Bíblia agora seguem o tema (antes eram claros no meio de um app escuro, e a seleção da Bíblia ficava ilegível). axe-core: 0 violações em claro e escuro.
- **Foco do teclado:** o ciclo do Tab dentro dos diálogos é feito à mão (o Chrome inclui áreas roláveis e `<summary>` na ordem e o foco escapava).

## Grafo travado (round 11)

Causas medidas com 136 notas / 1005 ligações:

1. **O layout era recalculado a cada movimento do mouse** (a lista de nós visíveis era recriada a cada render e invalidava o cache): ~220 iterações O(n²) por hover.
2. **`backdrop-filter: blur()` no fundo dos diálogos** forçava o navegador a re-borrar a tela inteira a cada repaint: hover 47 ms → **16,7 ms (60 fps)** só ao removê-lo. Vale para todos os diálogos (digitar, rolar, animar).

Mudanças: layout com **d3-force** (repulsão por quadtree, mola, colisão por raio; roda uma vez, determinístico) em vez de Fruchterman–Reingold artesanal; todas as ligações em um único `<path>`; destaque do mouse feito por classes direto no DOM (sem re-render); rótulos só dos 16 nós mais ligados + vizinhos + nota aberta (todos ao aproximar); **zoom (roda) e arrasto**; modo **Vizinhança da nota aberta** (até 2 ligações), que é o que de fato se lê num vault de 136 notas. Dependência nova: `d3-force` (+ tipos).

## Abrir último trabalho (round 12)

- **Botão "Abrir último trabalho"** na barra superior e **"Continuar: <livro>"** na tela vazia (com "capítulo · página" embaixo). Abre o livro mais recente já no **capítulo e na página** onde a leitura parou.
- **Onde fica a lista:** `recentes.json` na pasta de dados do APP (no Windows, `%APPDATA%\<identificador do app>\`), até 8 projetos, mais recente primeiro, sem duplicar a mesma pasta (barras e caixa ignoradas). Não usa o navegador. Arquivo ilegível vira `recentes.json.bak` e a lista recomeça.
- **Capítulo e página** continuam em `<livro>/.livro/projeto.json` (seção `leitura`) e são lidos na hora de abrir: nunca ficam defasados em relação ao livro.
- **A última página virada é gravada ao fechar a janela e ao trocar de livro** (antes havia 700 ms de espera, e fechar logo depois perdia a página).
- **Pasta movida/apagada ou unidade desconectada:** o botão explica o que houve em vez de falhar calado; abrir outro livro com texto não salvo salva o capítulo antes.

## Estrutura real do vault e recaps por capítulo (round 13)

Estrutura enviada pelo autor (`01-Biblias`, `01-Governanca`, `02-Trilogia`, `03-Mundo`, `04-Personagens/Fichas`, `05-Plots`, `06-Mapas`, `07-Revisao/{Arcos,Logs/L1…}`, `Arcos`) passou a ser um teste automático.

- **`07-Revisao/Logs` = recaps por capítulo.** A detecção só olhava o primeiro nível e deixava os 42 `Capitulo-NN` como "registros" (daí os 1618 registros); agora olha dentro das pastas mapeadas e `Logs` vira *capítulo*. **`01-Governanca`** (decisões, movimentações, inventário) vira *registro*. `02-Trilogia`, `Arcos` ficam como nota comum. `Indice` e `README` nunca são recap/registro/entidade. Botão **"Reanalisar pastas"** em *Dados do projeto* refaz a detecção sem trocar de pasta.
- **`chNN.xhtml` ⇄ `Capitulo-NN.md` casam pelo número.** A memória das análises passa a encontrar o recap do autor de cada capítulo anterior; a linha do tempo usa o número quando a nota não tem `ordem:`.
- **Recap gerado pela IA nunca substitui uma nota do autor.** Em nota existente do autor vai um bloco delimitado (`<!-- recap-do-app:inicio/fim -->`, "Recap do app — revise"), regravado sozinho nas próximas vezes e sem tocar no resto. Nota nova segue a convenção do vault (mesma pasta, prefixo e zeros: `Capitulo-07`); nota criada pelo app (`gerado-por: editor-de-livros`) continua sendo substituída por inteiro.
- Pastas novas mantêm a grafia do vault (`07-Revisao/Logs`, não `07-revisao/logs`).

## Limites conhecidos

- Provedores novos seguem a documentação pública, mas **não foram testados com chaves reais**; a leitura do catálogo está coberta por testes com respostas de exemplo. Anthropic ficou de fora: usa protocolo próprio (`x-api-key`), não o formato OpenAI.
- Em modo `extracted`, EPUB 2 é reempacotado como EPUB 2 (com aviso); não há conversão automática para EPUB 3, nem troca de capa ou série.
- A ordem dos capítulos no modo "pasta" é a do painel (ordem natural dos nomes), não um `spine` editável.
- O Kindle Previewer/KDP não foram executados.

---

# Rodada 3 — timeouts e uso da skill dentro do app

## Timeouts (causa e correção)

Toda análise era **uma requisição única, sem streaming, com teto fixo de 90 s para a resposta inteira**. Qualquer modelo que demorasse (fila em plano gratuito, modelo de raciocínio) estourava o teto mesmo funcionando; só modelos rápidos, como o DeepSeek Flash da NVIDIA, passavam. Agora (`src-tauri/src/chat.rs`):

- Resposta em **streaming (SSE)**. Limites: espera na fila 300 s, **silêncio** entre pedaços 120 s, teto absoluto 900 s. Enquanto o servidor envia algo (inclusive o raciocínio), a análise continua.
- Progresso ao vivo na interface (fila → raciocínio → escrita) e botão **Cancelar**.
- Mensagens específicas (fila cheia, servidor parou de enviar, limite de raciocínio) em vez de "excedeu 90 s".
- Blocos `<think>…</think>` são removidos; provedores que ignoram `stream` continuam funcionando.
- Testado com um servidor SSE local simulado (fluxo lento mas progredindo de 4 s com limite de inatividade de 2 s completa; silêncio, fila, cancelamento, JSON puro e erro HTTP).
- **Não testado com chaves reais** dos provedores. Se ainda houver timeout, o log (`IA: falha provider=… elapsed=…`) diz em qual fase.

## Mapa: o que da skill entrou no app

| Item da skill | No app |
|---|---|
| 20 padrões anti-IA (`analise.md`) | 6 detectados por regra no **Diagnóstico instantâneo** (#1, #2, #3, #8, #11, #15) como candidatos, com densidade por 3 mil palavras vs. meta do gênero; os outros 14 na análise por IA **Varredura anti-IA** |
| Metas por gênero (anti-IA, diálogo %, limiar Genesis) | Seletor de **gênero por projeto**; metas aplicadas ao diagnóstico e aos prompts |
| Diálogo: % por gênero, abuso de tag (>40%) | Diagnóstico instantâneo + perfil **Diálogo** |
| Ritmo (variação de frase, aceleração/desaceleração) | Diagnóstico instantâneo (heurística por tamanho de frase) |
| Hook Craft (abertura proibida) | Diagnóstico instantâneo (clima, acordar, backstory, abstração) + perfil **Hook e pull** |
| Genesis Score V3.7 | Perfil **Genesis Score**: a IA dá as 7 notas com evidência; **o app calcula** Floor×0,4 + Média×0,6 e compara com o limiar do gênero |
| `/quickscan`, `/voz`, `/dialogo`, `/hook`, `/caos`, `/revisao-sistemica` | Perfis novos (11 no total), com os módulos da skill dentro do prompt |
| Níveis de modelo Rápido/Padrão/Máximo | Dica exibida por perfil |
| Tabela de equivalências / suavização (`SKILL.md`) | Modo **tradução**: alertas de possível suavização (opt-in) |
| `corrigir_fluidez_ptbr.py` (69 regras) | Modo tradução: viram **sugestões antes/depois** com botão Aplicar (nunca em lote) |
| `glossario.json` (`producao.md`) | Lido da pasta do projeto e enviado à IA (nomes e termos protegidos) |
| `cenas-eroticas.md` | Opção "contém cenas adultas": preserva intensidade e consistência de registro; sem lista de termos explícitos no código |
| `producao.md` (EPUB) | Exportar EPUB (rodada 2) |

## Não implementado (decisão e motivo)

- **Comparação 1:1 com o EPUB original** (`/revisar-traducao`): o app não tem tela para o original; o perfil analisa só o PT-BR e pede conferência. Exige UI de dois documentos.
- **Sistema de blocos / LOG-RECAP / `/status`, bíblia de personagens, linha do tempo, mapa de fios:** exigem memória do projeto entre capítulos (arquivo `.recap` por projeto). É o maior ganho restante, mas é uma feature própria.
- **`/audiolivro` (ElevenLabs), `/sinopse`, `/capa`, `/persona`, `/pesquisar`, `/serie`:** perfis de criação sem manuscrito de entrada; baixa prioridade para um editor.
- **Remoção de watermark/DRM** descrita em `producao.md`: deliberadamente fora.
- Anti-IA por regra tem **falsos positivos** (ex.: "dançar" literal) e não substitui a leitura.

---

# Rodada 4 — glossário, duas telas, caderno (memória) e marca d'água

## Entregue

- **Glossário na interface:** criar/editar `glossario.json` (não traduzir, traduções fixas, personagens, equivalências de linguagem crua), com **sugestão automática de termos** lidos dos capítulos (nomes próprios e termos com apóstrofo). Escrita atômica e só dentro da pasta do projeto; campos extras do arquivo são preservados.
- **Tradução em duas telas:** original (EPUB ou pasta) à esquerda com rolagem acompanhando a página, editor à direita. Mapeia capítulos pelo nome do arquivo (ou posição). Checagem determinística de fidelidade (parágrafos, falas, expansão de palavras, omissões/adições por parágrafo, termos do glossário, preservação de linguagem crua) e o perfil **Fidelidade 1:1** por IA, que recebe original e tradução.
- **Caderno do livro (memória estilo Obsidian):** notas em Markdown com `[[wikilinks]]` e frontmatter YAML em `<projeto>/.livro-notas/` (abre como vault no Obsidian; fora do EPUB). Índice com apelidos, backlinks, links quebrados (clique cria a nota), menções por capítulo, renomear reescrevendo os links, grafo, linha do tempo. **LOG-RECAP por IA** (formato `/fechar-bloco` da skill) com revisão antes de gravar; cria/atualiza personagens, lugares e fios, e fecha fios. A memória (recaps anteriores + notas dos elementos citados + fios abertos) é injetada nos perfis que a usam; novo perfil **Continuidade**.
- **Marca d'água do próprio livro:** página de licença visível (nome + código) e marca forense invisível repetida nos capítulos; **leitura de volta** para identificar o exemplar de um EPUB vazado; registro de exemplares em `.livro-exemplares.json`. epubcheck 5.2.1: 0 erros com a marca aplicada.

## Decisão sobre "Watermark e DRM"

- **Implementado:** marca d'água/identificação de exemplares do **seu próprio livro**.
- **Não implementado:** *remover* marca d'água ou DRM de EPUBs de terceiros (descrito em `producao.md`): contorna proteção de obras alheias.
- **DRM de verdade não é possível num app local:** Adobe ACS4, Readium LCP e Kindle exigem servidor de licenças ou a própria loja. Na KDP é a opção "Habilitar DRM" ao publicar. A marca forense **não impede cópia** e pode ser removida por quem reescrever o texto.

## Limites conhecidos

- Chaves de API reais e o app Tauri completo não foram executados neste ambiente; as telas foram verificadas com os comandos simulados.
- Menções de personagens por título/apelido: não distinguem homônimos nem pronomes.
- Fidelidade por parágrafo só funciona quando original e tradução têm o mesmo número de parágrafos; as contagens por capítulo sempre valem.
- A marca forense usa caracteres de largura zero: pode afetar a busca e alguns leitores de tela; é opcional.
