# book_epub — núcleo de EPUB

Biblioteca Rust (sem dependência do Tauri) que inspeciona, valida e empacota EPUB 3. O app desktop a chama pelos comandos `epub_inspect`, `epub_validate` e `epub_build`.

## Dois modos (escolhidos pela pasta do projeto)

| Pasta contém | Modo | O que acontece |
|---|---|---|
| `META-INF/container.xml` + OPF (EPUB extraído) | `extracted` | Reempacota fielmente. Só o que está no manifest entra. Só os metadados que você alterar mudam no OPF (edição por intervalo de bytes; o resto do arquivo fica intacto). `dcterms:modified` é atualizado. |
| Apenas XHTML/HTML, CSS e imagens | `loose` | Gera `content.opf` (EPUB 3), `nav.xhtml`, `toc.ncx` (Kindle), página e imagem de capa, e uma folha de estilo base se o projeto não tiver CSS. Capítulos entram na ordem recebida (ou ordem natural dos nomes). Título do sumário vem do `<h1>-<h3>` ou das classes `chapter-number`/`chapter-title` da skill. |

Em ambos: `mimetype` é o primeiro item do ZIP e sem compressão; gravação atômica (arquivo temporário + rename); backups (`*.backup-*`), arquivos ocultos, links simbólicos e `.epub` nunca entram no pacote; `&nbsp;` e outras entidades HTML são convertidas para o equivalente numérico (XML não as define).

## Validação (bloqueia a geração em caso de erro)

container/OPF, identificador, título, idioma, `dcterms:modified`, manifest × arquivos, spine, documento de navegação (links existem), NCX, capa, XHTML bem formado (UTF-8, raiz `<html xmlns>`, `<title>`), imagens e CSS referenciados existem, scripts e recursos remotos, nomes duplicados/com espaços. Não substitui o **epubcheck** oficial: rode-o antes de enviar a uma loja.

Não implementado de propósito: remoção de DRM/`encryption.xml` e de marcas d'água de terceiros (o pacote apenas avisa que `encryption.xml` existe e o mantém).

## Testar fora do app

```
cd packages\epub
cargo test
cargo run --release --example epubtool -- inspect  "D:\caminho\da\pasta"
cargo run --release --example epubtool -- validate "D:\caminho\da\pasta" "Título" "Autor"
cargo run --release --example epubtool -- build    "D:\caminho\da\pasta" "D:\saida\livro.epub" "Título" "Autor" "D:\capa.jpg"
```

Verificado com o epubcheck 5.2.1 (0 erros) nos dois modos.
