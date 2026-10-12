# Premissas de produto

## Problema que resolvemos

Quem edita livros em XHTML/EPUB precisa alternar entre código, leitor, gerenciador de arquivos e ferramentas de revisão. O produto reúne isso em uma estação local: leitura confortável, edição estruturalmente segura, histórico recuperável e assistência editorial opcional.

## Princípios

1. **Local-first.** O livro fica na máquina da pessoa; nenhuma cópia é enviada sem ação explícita.
2. **O original é recuperável.** Toda gravação destrutiva pede confirmação e cria backup verificável.
3. **Sugestão antes de reescrita.** IA propõe um diff ancorado no trecho; aplicação automática é uma opção explícita, nunca o padrão.
4. **EPUB é publicação, não pasta comum.** O app preserva manifesto, spine, navegação, CSS, imagens e metadados.
5. **Prévia não é fonte da verdade.** A fonte da verdade é o XHTML estruturado; o leitor é uma representação segura dele.
6. **Acessível e auditável.** Atalhos, contraste, leitor de tela, validação, histórico e relatórios fazem parte do produto.

## Escopo inicial (MVP)

- Projeto de pasta XHTML: índice, ordem manual, abrir/editar/salvar, busca e substituição literal, desfazer/refazer e validação XHTML.
- Projeto EPUB: importar para área de trabalho, listar capítulos pelo spine, editar XHTML e exportar um novo `.epub`.
- Leitor paginado: família/tamanho de fonte, tema, espaçamento, largura, navegação de páginas e indicador de capítulo/progresso.
- Backups: confirmação de gravação, snapshot com data/hora, retenção configurável e restauração.
- IA da página aberta: análise e sugestões com diff, usando provedor selecionado e chave guardada localmente.

## Fora do MVP

- Sincronização em nuvem e colaboração simultânea.
- Converter PDF/DOCX para EPUB com fidelidade editorial.
- Edição WYSIWYG de layouts complexos/fixed-layout.
- Garantia de que a paginação será idêntica a todos os modelos de Kindle.
- Alterar capítulos em massa via IA sem revisão humana.

## Critérios de aceite essenciais

- Nenhuma gravação no original ocorre sem confirmação e backup concluído.
- Um EPUB importado pode ser exportado sem perder recursos não editados nem alterar a ordem do spine.
- O app bloqueia a exportação final quando encontrar XHTML inválido ou referência quebrada, explicando em qual arquivo.
- Uma sugestão de IA mostra escopo, motivo, trecho original e alteração proposta antes de aplicar.
- Ao reabrir um projeto, o app recupera estado, backups e documentos modificados.

