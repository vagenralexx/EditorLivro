# Superfície visual e XHTML

## Decisão de produto

O **XHTML é o formato de persistência**, não a superfície principal de edição. A pessoa trabalha em uma página tipográfica editável, com a sensação de uma tela de e-reader e ferramentas de texto. Ao digitar ou formatar, o corpo XHTML em memória é atualizado; ao salvar, o arquivo original só é substituído depois de confirmação e criação de backup.

## Modos

| Modo | Finalidade | Estado no MVP |
| --- | --- | --- |
| Editar | Página visual, cursor, seleção e barra de formatação | Implementado |
| Ler | Mesmo conteúdo, sem cursor, com paginação real | Próxima etapa |
| Estrutura | Inspecionar XHTML, classes e avisos de validação | Avançado, oculto por padrão |

Não haverá textarea de XHTML no fluxo editorial normal. O modo Estrutura existe para diagnóstico e não para substituir a experiência visual.

## Contrato de preservação

1. Cabeçalho, metadados, atributos do elemento `html` e atributos do elemento `body` são mantidos.
2. O canvas edita o conteúdo do `body` e usa marcação XHTML ao persistir.
3. Classes, imagens e semântica EPUB existentes devem permanecer sempre que não forem atingidas pela edição.
4. O produto precisa alertar antes de uma operação visual que não consiga representar um elemento complexo (SVG, JavaScript, formulários, layout fixo ou mídia incorporada).
5. Cada gravação gera backup datado ao lado do original.

## Paginação de leitura e edição

O capítulo permanece XHTML fluido. O app cria uma **paginação de visualização** com base no tamanho de fonte, largura e altura atuais — exatamente como leitores EPUB fazem. Portanto, mudar a fonte pode alterar o número da página e nenhuma quebra artificial é gravada no arquivo.

- Edição e leitura usam o mesmo fluxo de páginas.
- Setas movem entre as páginas visíveis do capítulo.
- A análise por IA recebe apenas os blocos visíveis na página atual.
- O rodapé indica página atual e total refluído.

## Segurança ao navegar pelo livro

Ao selecionar outro capítulo — inclusive pela seta após a última página — o aplicativo verifica alterações pendentes. Em vez de um alerta técnico, ele mostra o modal **Salvar edições e continuar**. Salvar cria o backup automaticamente e abre o próximo capítulo; a pessoa também pode cancelar ou descartar de forma explícita.

## Tema de edição

O alternador de lua na barra superior ativa um tema escuro para toda a estação de edição, incluindo biblioteca, assistente e página tipográfica. O brilho continua sendo um ajuste independente da página, para simular o leitor sem mudar o brilho físico do monitor.

## Referência: WordCraft

O [WordCraft](https://github.com/storytold/wordcraft) ajuda como referência de arquitetura, não como componente a ser incorporado. Ele é outro aplicativo, escrito em Rust com interface nativa e modelo de documento próprio; conectá-lo ao Tauri/React introduziria dois editores concorrentes.

Ideias adotáveis:

- comandos nomeados para ações de edição, desfazer e refazer;
- separação entre modelo do documento, layout/paginação e interface;
- sugestões de IA como mudanças revisáveis, nunca como gravação direta;
- modo de foco, navegação estrutural e revisões como recursos independentes do canvas.

## Próximas camadas

1. Paginação física no modo Ler, independente do canvas contínuo de edição.
2. Histórico de comandos e painel de alterações sugeridas.
3. Mapeamento seguro de imagens e CSS relativos quando o projeto é uma pasta EPUB extraída.
4. Importação/exportação EPUB com validação de manifesto, spine e XHTML.
