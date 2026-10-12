# EPUB, backups e segurança

## EPUB como arquivo de publicação

Um EPUB é um ZIP com regras. A edição precisa preservar `mimetype`, `META-INF/container.xml`, OPF, manifest, spine, nav/NCX, XHTML, CSS, fontes, imagens, mídia e metadados. Um capítulo isolado não é suficiente para definir a ordem de leitura nem validar referências.

## Importar com segurança

- Aceitar apenas `.epub`, verificar assinatura/estrutura mínima e impor limites de tamanho, quantidade de arquivos e tamanho descompactado.
- Bloquear ZIP Slip: todo caminho extraído deve permanecer dentro do workspace calculado.
- Não executar scripts, handlers, objetos externos ou conteúdo ativo vindo do livro.
- Ler o OPF indicado pelo `container.xml`; não adivinhar capítulos por nome de arquivo.
- Avisar quando o livro for DRM/protegido ou não puder ser interpretado sem violar seus controles. O produto não remove DRM.

## Reempacotar corretamente

1. Validar XHTML, manifest, spine, links e recursos.
2. Escrever `mimetype` como primeiro item do ZIP e sem compressão.
3. Escrever os demais recursos mantendo caminhos relativos e compressão apropriada.
4. Executar EPUBCheck e apresentar erros/avisos antes da exportação final.
5. Gerar um relatório com hashes, documentos alterados e resultado da validação.

## Política de salvamento

### Padrão para XHTML de pasta

`Salvar` abre uma confirmação com: arquivo, resumo do diff, destino e backup que será criado. Ao confirmar, o app cria snapshot e somente então grava de forma atômica (arquivo temporário → fsync quando disponível → rename/substituição).

### Padrão para EPUB

`Exportar EPUB` sempre cria um novo arquivo, por exemplo `titulo.editado.2026-10-11.epub`. A ação "substituir original" fica escondida atrás de confirmação reforçada e backup completo.

### Backups

- Organização sugerida: `.book-editor/backups/<timestamp>/<documento-ou-publicacao>`.
- Manifesto de backup com hash SHA-256, tamanho, origem, motivo e versão do app.
- Retenção inicial: últimos 20 snapshots por projeto, com opção de manter marcos manuais indefinidamente.
- Tela de restauração com diff e opção de restaurar como cópia, evitando nova perda.

## Validações em camadas

1. XML/XHTML bem-formado.
2. Regras de estrutura e links locais.
3. Integridade do pacote EPUB (OPF/manifest/spine/nav).
4. EPUBCheck na exportação.
5. Prévia reflow e, para publicação, teste externo no Kindle Previewer/leitor-alvo.

