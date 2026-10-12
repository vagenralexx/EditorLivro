# Roadmap de implementação

## Fase 0 — Fundação e decisões (curta)

- Definir framework desktop por spike: abrir pasta, backup e exportar EPUB mínimo.
- Criar modelo de domínio, formato de projeto e estratégia de banco local.
- Reproduzir um EPUB real de teste em importação → exportação sem mudanças e comparar hashes/estrutura permitida.
- Critério: exportação válida e nenhum arquivo da origem é modificado.

Antes de iniciar a Fase 1, executar e documentar os spikes [[02-Arquitetura/Bibliotecas-e-Spikes#Spikes obrigatórios|S1–S4]]. Nenhuma biblioteca editorial entra como dependência definitiva apenas por popularidade.

## Fase 1 — Editor XHTML seguro

- Biblioteca de capítulos, editor fonte, prévia sandboxed e preferências de leitura.
- Undo/redo, busca literal com preview, validação contínua e navegação por capítulo.
- Save dialog + backup + escrita atômica + restauração.
- Critério: editar uma pasta de capítulos por uma sessão inteira sem perda e com reversão testada.

## Fase 2 — EPUB completo

- Importador com OPF/manifest/spine/nav, workspace extraído e recursos preservados.
- Exportação para novo EPUB, relatório e EPUBCheck.
- CSS/lint de compatibilidade Kindle e presets de prévia reflow.
- Critério: EPUB reimportado abre corretamente, passa validação e mantém capa/navegação/imagens.

## Fase 3 — IA editorial na página aberta

- Tela de provedores, keychain, testes de conexão e consentimento de envio.
- Seleção/parágrafo/capítulo como escopo; streaming, cancelamento, diff e aplicação reversível.
- Modos: revisão de linha, quickscan e brainstorm.
- Critério: nenhuma alteração IA é aplicada sem ação explícita e toda aplicação pode ser desfeita.

## Fase 4 — Inteligência de livro

- Perfil editorial, glossário, bíblia de personagens, recaps por bloco e mapa de fios.
- Diagnóstico de capítulo, Genesis Score e análise de coerência entre capítulos.
- Revisão de tradução com alinhamento de parágrafos e fluxo de aprovação.

## Fase 5 — Qualidade e publicação

- Acessibilidade, internacionalização, telemetria opt-in, testes de regressão com corpus EPUB e desempenho para livros longos.
- Assistente de pré-publicação: EPUBCheck, relatório, checklist Kindle e exportação de pacote editorial.

## Backlog priorizado

1. Backup confirmado e escrita atômica.
2. Parser/validador XHTML e editor fonte confiável.
3. Importação/exportação EPUB sem perdas.
4. Histórico/diff/restauração.
5. Prévia Kindle configurável em sandbox.
6. Painel de IA com sugestões por diff.
7. Regras da skill convertidas em perfis e análises estruturadas.

