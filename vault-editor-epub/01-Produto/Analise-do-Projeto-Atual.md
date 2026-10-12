# Análise do protótipo atual

Arquivo analisado: `editor_xhtml_profissional_pagina_o.html`.

## O que já funciona bem

- Abre uma pasta com `.xhtml`/`.html`, ordena os arquivos e carrega o primeiro capítulo.
- Mostra uma superfície de leitura com colunas horizontais e controles de página; é uma boa direção para a experiência Kindle.
- Mantém uma visão dividida entre prévia visual e fonte XHTML.
- Preserva, em casos simples, `doctype`, atributos de `<html>`, `<head>` e conteúdo do `<body>`.
- Oferece formatação básica, travessão, divisor de cena, diálogo, contador de palavras, tema escuro, corretor do navegador e busca/substituição.
- Quando a File System Access API está disponível, consegue escrever de volta no arquivo aberto.

## Limitações que impedem uso editorial confiável

| Área | Evidência no protótipo | Consequência | Direção |
|---|---|---|---|
| Persistência | fallback de upload apenas baixa uma cópia | o usuário pode achar que salvou no original | app desktop com acesso nativo e estado explícito |
| Backup | `saveCurrentFile()` grava diretamente | não há confirmação, backup ou restauração | snapshot antes da escrita + confirmação |
| XHTML | HTML é manipulado via `innerHTML` e regex | pode normalizar/perder XML, namespaces, entidades ou atributos | parser/serializador XML e validação |
| EPUB | não há importação, OPF, spine, nav nem reempacotamento | não pode tratar um livro como publicação | modelo `BookProject` e pipeline EPUB |
| Edição | `document.execCommand` está obsoleto e a prévia é editável | marcação inválida/inesperada pode entrar no capítulo | editor baseado em transações e esquema XHTML |
| Busca/substituição | `RegExp(findText, 'g')` usa o texto do usuário como regex | caracteres especiais podem quebrar a busca ou alterar HTML | busca literal no texto/árvore, com preview |
| Segurança de conteúdo | capítulo é injetado diretamente na mesma página do app | script/event handlers de XHTML podem afetar o app | renderizar prévia em sandbox e sanitizar |
| Qualidade | não há desfazer/refazer persistente, diff, validação ou testes | mudança editorial fica difícil de auditar | histórico por documento + validação contínua |

## Observação sobre a "página do Kindle"

A paginação atual usa CSS columns. Isso é ótimo para uma simulação de leitura, mas o Kindle real repagina conforme aparelho, tamanho da tela, fonte, margens, hifenização e motor de renderização. A promessa correta é: **prévia reflow semelhante à leitura Kindle**, com presets e alertas de CSS incompatível; a verificação final continua sendo feita em Kindle Previewer/e-reader real.

## Diagnóstico

O protótipo é uma boa maquete de interface. Para se tornar produto, a prioridade é separar: modelo do documento, serviços de arquivo/EPUB, editor estruturado, leitor em sandbox e interface. Acrescentar EPUB ou IA antes dessa separação ampliaria risco de corrupção silenciosa de livros.

