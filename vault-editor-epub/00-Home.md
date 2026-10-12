# Editor de Livros EPUB

> Vault de produto e engenharia do editor local-first para XHTML e EPUB.

## Norte do produto

Editar capítulos de um livro com a sensação de leitura de um Kindle, preservando a estrutura editorial do arquivo e permitindo salvar com segurança no local de origem.

## Premissas acordadas

- Abrir e editar arquivos `.xhtml`/`.html` em pastas locais.
- Abrir um `.epub`, editar seus documentos internos e exportar um EPUB válido.
- Exibir uma prévia de leitura paginada, inspirada no Kindle, sem prometer reprodução pixel a pixel do dispositivo.
- Antes de gravar sobre o original, mostrar confirmação e assegurar a existência de backup.
- Integrar provedores de IA configuráveis (Groq, OpenAI/GPT e outros) para análise, revisão e sugestões da página aberta.
- Incorporar o conhecimento da skill editorial como regras versionadas e modos de análise, mantendo a decisão final com a pessoa autora/editora.

## Navegação

- [[01-Produto/Premissas]] — escopo, princípios e critérios de aceite.
- [[01-Produto/Analise-do-Projeto-Atual]] — o que o protótipo já faz e seus limites.
- [[02-Arquitetura/Arquitetura-Alvo]] — desenho técnico recomendado.
- [[02-Arquitetura/Estrutura-do-Repositorio]] — módulos, contratos e árvores de pastas decididos antes do código.
- [[02-Arquitetura/Bibliotecas-e-Spikes]] — bibliotecas candidatas e provas técnicas obrigatórias.
- [[02-Arquitetura/EPUB-e-Seguranca]] — importação, edição, backup e exportação de EPUB.
- [[03-IA/Integracao-IA]] — provedores, privacidade e capacidades editoriais.
- [[04-Roadmap/Roadmap]] — entregas em ordem de valor e risco.
- [[05-Decisoes/ADR-001-Desktop-Local-First]] — primeira decisão arquitetural.
- [[05-Decisoes/ADR-002-Esforco-de-Raciocinio-por-Modelo]] — seletor de esforço de raciocínio por modelo.

## Estado atual

`editor_xhtml_profissional_pagina_o.html` é um protótipo web de arquivo único. A etapa seguinte não é adicionar recursos diretamente nele: é estabelecer um núcleo de documento confiável e uma camada desktop local-first.

## Implementação iniciada

- `apps/desktop` foi criado com Tauri + React e usa o ícone do produto.
- A primeira tela abre uma pasta real, localiza capítulos `.xhtml`/`.html`, permite alterná-los e exibe a fonte e a prévia de leitura sem conteúdo demonstrativo.
- O salvamento do XHTML exige confirmação e cria um arquivo `nome.xhtml.backup-<timestamp>` antes de substituir o original.
- NVIDIA NIM, Groq e OpenAI são configurados em **Configurações**. A chave vai ao Gerenciador de Credenciais do Windows; ela não é gravada no projeto, no EPUB ou no código-fonte. A tela permite salvar e testar a conexão, e a análise usa o capítulo real aberto.
- `packages/editorial-rules`, `packages/ai`, `packages/domain` e `packages/platform-contracts` ganharam os primeiros contratos. Os demais módulos foram criados como fronteiras explícitas, prontos para implementação incremental.

## Limite da versão de teste atual

O fluxo funcional desta entrega é **pasta XHTML → edição → backup → salvamento → análise por IA**. A abertura, reempacotamento e validação de um `.epub` ainda é o próximo módulo; não há botão que finja suporte completo enquanto ele não existir.

