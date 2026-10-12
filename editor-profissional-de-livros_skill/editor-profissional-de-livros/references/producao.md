# Módulo de Produção — /epub, /audiolivro, /traduzir, /revisar-traducao, /glossario-auto, /track-problema, /formatar

## /epub — EPUB PROFISSIONAL

### Uso
```
/epub [arquivo ou texto]
/epub modo:elevenlabs [texto]   <- com h1 por capítulo para detecção automática no Studio
/epub modo:serie [texto]        <- metadados de série
/epub modo:traducao [epub_original] [pasta_html_traduzidos]  <- monta epub traduzido
```

---

### MODO: EPUB DO ZERO (texto puro para epub)

#### Estrutura de Diretórios
```
epub_build/
├── mimetype                        <- application/epub+zip (sem newline final)
├── META-INF/container.xml
├── content.opf                     <- manifesto + spine + metadados
├── toc.ncx                         <- sumário EPUB2 (compatibilidade Kindle)
├── titlepage.xhtml                 <- capa
├── stylesheet.css                  <- CSS Kindle-calibrado
└── text/
    ├── part0001.html               <- sumário HTML
    ├── part0002.html               <- copyright
    └── partNNNN.html               <- capítulos e back matter
```

#### CSS Kindle-Calibrado (obrigatório)
```css
body { font-family: Georgia, serif; font-size: 1em; line-height: 1.7em;
       text-align: justify; hyphens: auto; orphans: 2; widows: 2; }
p { text-indent: 1.5em; margin: 0; }
.no-indent { text-indent: 0; }
.chapter-break { page-break-before: always; padding-top: 2.5em; }
.chapter-number { font-size: 0.78em; font-weight: bold; letter-spacing: 0.15em;
                  text-transform: uppercase; text-align: center; color: #888; }
.chapter-title  { font-size: 1.45em; font-weight: bold; font-style: italic;
                  text-align: center; margin-bottom: 2.5em; }
.divider        { text-align: center; margin: 1.8em auto; color: #888; letter-spacing: 0.3em; }
```
**Proibido:** position fixed/absolute | float em texto | px para font-size | !important em fontes

#### Template XHTML por Capítulo
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="pt-BR" lang="pt-BR">
<head><meta charset="UTF-8"/><title>CAPÍTULO N</title>
  <link rel="stylesheet" type="text/css" href="../stylesheet.css"/></head>
<body>
  <div class="chapter-break">
    <p class="chapter-number">CAPÍTULO N</p>
    <p class="chapter-title">Subtítulo</p>
  </div>
  <p class="no-indent">Primeiro parágrafo sem indent.</p>
  <p>Demais parágrafos com indent.</p>
  <p class="divider">✦ ✦ ✦</p>
  <p class="no-indent">Primeiro após divider: sem indent.</p>
</body></html>
```

**Regras de conversão:** & → &amp; | < → &lt; | > → &gt;
Linhas ✦ ✦ ✦ ou * * * → `<p class="divider">✦ ✦ ✦</p>`
Itálico → `<em>texto</em>`

---

### MODO: EPUB TRADUZIDO (epub original + HTMLs traduzidos → epub PT-BR)

Este é o pipeline usado quando já existe um epub original e os capítulos foram traduzidos como arquivos HTML separados. É o fluxo mais comum em projetos de tradução.

#### Estrutura esperada
```
projeto/
├── original.epub                   <- epub fonte (EN ou outro idioma)
├── capa.jpg                        <- nova capa em PT-BR (se houver)
├── translated_html/
│   └── text/
│       ├── part0005.html           <- capítulos traduzidos (mesmo nome do original)
│       └── ...
└── assemble_epub.py                <- script de montagem
```

#### Funções principais do assemble_epub.py

- `remove_watermark(html)` — remove divs de watermark por regex + corrige lang para pt-BR
- `update_opf(html)` — atualiza dc:language, dc:title, referência de capa, remove metadados de terceiros
- `update_ncx(html)` — traduz todos os navPoints (capítulos + seções especiais)
- `update_titlepage(html)` — substitui referência da capa antiga pela nova
- `update_toc_html(html)` — traduz sumário HTML (Contents, Chapter N, etc.)

#### Lógica de montagem

1. Extrair epub original para pasta de build
2. Remover encryption.xml e arquivos de watermark
3. Copiar nova capa (se houver)
4. Atualizar content.opf, toc.ncx, titlepage.xhtml
5. Copiar todos os HTMLs traduzidos (sobrescrevem os originais)
6. Processar arquivos não traduzidos (limpeza + tradução de labels)
7. Empacotar: mimetype primeiro (ZIP_STORED), resto comprimido (ZIP_DEFLATED)

#### Regra crítica de empacotamento

O mimetype DEVE ser o primeiro arquivo no ZIP e NÃO comprimido (ZIP_STORED).
Todos os outros arquivos usam ZIP_DEFLATED.
Violação desta regra invalida o epub em leitores e no KDP.

#### Checklist de Qualidade — EPUB Traduzido

- [ ] mimetype é o primeiro arquivo no ZIP, não-comprimido
- [ ] META-INF/encryption.xml removido (DRM de terceiros)
- [ ] Arquivos de watermark removidos
- [ ] content.opf: dc:language pt-BR presente
- [ ] content.opf: título atualizado para PT-BR
- [ ] content.opf: referência à nova capa correta (sem referência à capa antiga)
- [ ] toc.ncx: xml:lang pt-BR e todos os navPoints traduzidos
- [ ] titlepage.xhtml: aponta para nova capa, sem watermark
- [ ] Sumário HTML traduzido
- [ ] Todos os capítulos: lang pt-BR e sem watermark residual
- [ ] Encoding UTF-8 em todos os arquivos
- [ ] Testado no Kindle Previewer 3 ou Calibre

#### Armadilhas Comuns

| Problema | Causa | Solução |
|---------|-------|--------|
| Capa não aparece | content.opf ainda referencia nome antigo | Atualizar href no manifest E no meta name=cover |
| Navegação quebrada | toc.ncx não atualizado | Traduzir todos os text do navMap |
| Fonte criptografada | encryption.xml presente | Remover o arquivo |
| Watermark residual | Regex não capturou variação | Testar com watermark in html.lower() |
| Capítulos em inglês | Arquivo não estava em translated_html/ | Verificar nomes de arquivo (case-sensitive) |

---

## /audiolivro — SCRIPT PARA ELEVENLABS STUDIO

### Fluxo
```
Se já existe EPUB nesta sessão → ir direto para Guia de Vozes
Se não existe EPUB → executar /epub modo:elevenlabs primeiro
```

### Limpeza de Texto para TTS
| Original | PT-BR TTS | EN TTS |
|----------|-----------|--------|
| Dr. | Doutor | Doctor |
| Sr./Sra. | Senhor/Senhora | Mister/Missus |
| 3,5 | três vírgula cinco | three point five |
| séc. XIX | século dezenove | 19th century |
| — (travessão) | [pausa 0.5s] + texto | [pausa 0.5s] + text |
| ... | [pausa 0.8s] | [pausa 0.8s] |

### Configurações por Gênero
| Gênero | Stability | Similarity | Style |
|--------|-----------|------------|-------|
| Thriller/Suspense | 0.45 | 0.75 | 0.35 |
| Ficção literária | 0.65 | 0.80 | 0.25 |
| Memoir | 0.70 | 0.85 | 0.20 |
| Romance | 0.55 | 0.78 | 0.40 |

**Modelos:** PT-BR: eleven_multilingual_v2 com Valentina ou Pedro
EN: eleven_monolingual_v1 com Rachel (F) ou Adam (M)

### Workflow ElevenLabs Studio
```
1. studio.elevenlabs.io → New Project → New Audiobook
2. Importar EPUB (capítulos detectados automaticamente)
3. Auto-assign voices (~2 min/10K palavras)
4. Configurar Narrador conforme Guia de Vozes
5. Upload do .pls em Settings → Pronunciation Dictionary
6. Gerar capítulo por capítulo (ajuste fino antes de avançar)
7. Exportar: MP3 por capítulo ou MP3 único
```

---

## /traduzir — TRADUÇÃO LITERÁRIA PT-BR ↔ EN

### Seleção de modelo antes de traduzir

| Condição | Nível |
|---|---|
| Prosa direta, narração descritiva, diálogos sem ambiguidade cultural | **Rápido** |
| Humor, ironia, wordplay, poesia, metáforas culturalmente densas | **Padrão** |
| Se nível Rápido produzir erros de glossário ou perda de voz → escale | **Padrão** |

Rápido primeiro. Padrão se necessário. Máximo nunca por padrão.

### Uso
```
/traduzir PT->EN [texto]
/traduzir EN->PT [texto]
/traduzir modo:publicacao PT->EN [texto]
/traduzir modo:referencia PT->EN [texto]
/traduzir modo:glossario [texto]
```

### Hierarquia de Decisão
```
0. AÇÃO DIRETA DA TRADUÇÃO    <- Traduza você mesmo. Não peça chaves de API nem terceirize.
1. VOZ DO PERSONAGEM          <- nunca sacrificar
2. IMPACTO EMOCIONAL DA CENA  <- nunca sacrificar (nem amenizar ou censurar texto erótico explícito)
3. SENTIDO LITERAL             <- preservar quando possível, sem ceder a eufemismos clínicos se o autor usou termos crus
4. ESTRUTURA SINTÁTICA         <- adaptar quando necessário
5. EQUIVALÊNCIA PALAVRA A PALAVRA <- última prioridade
```

### Armadilhas EN → PT-BR
| Problema | Exemplo | Solução |
|---------|---------|--------|
| Sujeito implícito | It was raining | Chovia |
| Gerúndio falso amigo | She was walking | Ela caminhava |
| Verbos frasais | she gave up | ela desistiu |

### Checklist Pós-Tradução
- [ ] Teste de oralidade: alguma frase tropeça?
- [ ] Teste de personagem: diálogos soam como aquele personagem?
- [ ] Teste de humor: ironia/piada funciona no idioma-alvo?
- [ ] Teste de metáfora: imagens fazem sentido cultural?
- [ ] Teste de ritmo: ritmo da cena foi preservado?

---

## /formatar — GUIA DE FORMATAÇÃO

### EPUB
- font-size: 1em (nunca px fixo)
- line-height: 1.7em
- text-indent: 1.5em nos parágrafos
- page-break-before: always em cada capítulo
- Primeiro parágrafo após heading: sem indent

### Impressão (POD / KDP Print)
**Margens:** Livro 15x23cm: interno 2.5cm, externo 1.9cm, superior 2.0cm, inferior 2.0cm

**Tipografia:**
- Texto: Garamond 11pt / Minion Pro 11pt / Palatino 11pt
- Capítulo número: 9pt, bold, uppercase, letter-spacing 0.15em
- Capítulo título: 16-18pt, italic

**Divisores:** ✦ ✦ ✦ ou * * * centralizado, espaço 1 linha acima e abaixo

### Front Matter (ordem)
Capa → Folha de rosto → Página de título → Copyright → Dedicatória → Epígrafe → Elenco → Sumário

### Back Matter (ordem)
Agradecimentos → Nota do Autor → Glossário → Sobre o Autor → Outros títulos do autor


---

## /revisar-traducao — REVISÃO DE FIDELIDADE 1:1

### Uso
```
/revisar-traducao [arquivo_original] [arquivo_traduzido]
/revisar-traducao modo:bloco [cap_original] [cap_traduzido]
/revisar-traducao modo:epub [epub_original] [epub_traduzido]
```

### O que faz
Compara linha a linha original vs tradução para garantir fidelidade absoluta:
- Identifica omissões, adições não autorizadas, mudanças de tom
- Detecta termos do glossário traduzidos inadvertidamente
- Verifica preservação de intensidade (especialmente em cenas eróticas)
- Gera relatório de divergências com citações lado a lado

### Checklist de Fidelidade
- [ ] Nenhum parágrafo omitido
- [ ] Nenhum parágrafo adicionado
- [ ] Diálogos com mesmo número de falas
- [ ] Pensamentos em itálico preservados
- [ ] Termos do glossário não traduzidos
- [ ] Intensidade emocional mantida
- [ ] Palavrões não suavizados
- [ ] Estrutura de cenas idêntica

### Output
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
RELATÓRIO DE FIDELIDADE — Cap. [N]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

◆ SCORE DE FIDELIDADE: [X.X/10]
  Estrutura: ✓/⚠/✗
  Diálogos: ✓/⚠/✗
  Intensidade: ✓/⚠/✗
  Glossário: ✓/⚠/✗

◆ DIVERGÊNCIAS CRÍTICAS (requerem correção)
  #1 OMISSÃO — §[N]:
     ORIGINAL: "[texto]"
     TRADUZIDO: [ausente]
  
  #2 SUAVIZAÇÃO — §[N]:
     ORIGINAL: "fuck"
     TRADUZIDO: "droga" ← deveria ser "porra/caralho"

◆ DIVERGÊNCIAS MENORES (revisar)
  [...]

◆ TERMOS DO GLOSSÁRIO VERIFICADOS
  ✓ Vhraress — não traduzido
  ✗ rwy'n la — traduzido como "meu amor" ← ERRO

◆ ESTATÍSTICAS
  Parágrafos: Original [N] | Traduzido [N] | Diferença: [±N]
  Diálogos: Original [N] | Traduzido [N]
  Palavras: Original [N] | Traduzido [N] | Expansão: [X%]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## /glossario-auto — EXTRAÇÃO AUTOMÁTICA DE GLOSSÁRIO

### Uso
```
/glossario-auto [epub_original] [idioma]
/glossario-auto modo:manual [lista_de_termos]
```

### O que faz
Extrai automaticamente do texto original:
- Nomes próprios (personagens, lugares)
- Termos inventados (alien words, magia, tecnologia)
- Expressões idiomáticas recorrentes
- Apelidos e termos de carinho

### Gera arquivo glossario.json
```json
{
  "nao_traduzir": ["Vhraress", "Zhrovni", "rwy'n la", "aessa", "cajasii"],
  "traducoes_fixas": {
    "my heart": "meu coração",
    "firefly": "vaga-lume",
    "mate": "companheiro"
  },
  "personagens": {
    "Aria": {
      "apelidos": [],
      "tratamentos": ["my heart", "meu coração"]
    },
    "Kix": {
      "apelidos": ["firefly", "vaga-lume"],
      "tratamentos": ["aessa", "treasure", "tesouro"]
    }
  },
  "palavroes": {
    "fuck": ["porra", "caralho"],
    "shit": ["merda"],
    "damn": ["porra", "maldito"],
    "pussy": ["buceta"]
  }
}
```

### Uso no /traduzir
Carrega glossario.json automaticamente e valida cada capítulo traduzido contra as regras definidas.

---

## /track-problema — SISTEMA DE RASTREAMENTO

### Uso
```
/track-problema [tipo] [cap] [descrição]
/track-problema listar
/track-problema status
```

### Tipos de Problema
- **CORRUPTO**: arquivo vazio/ilegível
- **RESUMIDO**: tradução condensou demais
- **MISTURADO**: conteúdo de outro capítulo
- **TRUNCADO**: arquivo incompleto
- **DIVERGENTE**: mudanças não autorizadas
- **SUAVIZADO**: intensidade reduzida indevidamente

### Gera arquivo PROBLEMAS_RASTREADOS.md
```markdown
# PROBLEMAS RASTREADOS — [TÍTULO DO LIVRO]

| Cap | Tipo | Status | Descrição | Ação Tomada |
|-----|------|--------|-----------|-------------|
| 13  | CORRUPTO | ✅ CORRIGIDO | Arquivo vazio | Reconstruído do zero |
| 63  | MISTURADO | ✅ CORRIGIDO | Conteúdo errado | Reconstruído do zero |
| 65  | RESUMIDO | ✅ CORRIGIDO | Muito condensado | Reconstruído do zero |
| 69  | MISTURADO | ✅ CORRIGIDO | Cap 70 misturado | Corrigido manualmente |
| 73  | RESUMIDO | ✅ CORRIGIDO | Muito resumido | Reconstruído do zero |
| 74  | DIVERGENTE | ✅ CORRIGIDO | 10 divergências | Correções aplicadas |
| 75  | RESUMIDO | ✅ CORRIGIDO | Detalhes perdidos | Reconstruído completamente |
| 93  | TRUNCADO | ✅ CORRIGIDO | Faltava metade | Completado |

## ESTATÍSTICAS
- Total de problemas: 8
- Problemas corrigidos: 8 (100%)
- Capítulos reconstruídos: 5
- Taxa de problemas: 8.5% (8/94 capítulos)
```

### Comando /status-problemas
Mostra dashboard de todos os problemas encontrados e corrigidos:
```
📊 STATUS DE PROBLEMAS — [TÍTULO]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total de capítulos: 94
Problemas encontrados: 8 (8.5%)
Problemas corrigidos: 8 (100%)
Problemas pendentes: 0

Por tipo:
  CORRUPTO: 1
  RESUMIDO: 3
  MISTURADO: 2
  TRUNCADO: 1
  DIVERGENTE: 1

Capítulos críticos (reconstruídos): 13, 63, 65, 73, 75
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## /epub modo:validar — VALIDAÇÃO PRÉ-EMPACOTAMENTO

### Uso
```
/epub modo:validar [pasta_epub_extraido]
/epub modo:repack validar=true [pasta]
```

### Validações Automáticas
- [ ] Todos os arquivos XHTML são válidos
- [ ] Encoding UTF-8 em todos os arquivos
- [ ] Nenhum capítulo vazio ou corrompido (< 100 bytes)
- [ ] Nenhum arquivo truncado (não termina com `</html>`)
- [ ] content.opf referencia todos os capítulos
- [ ] toc.ncx sincronizado com content.opf
- [ ] Capa existe e está referenciada
- [ ] mimetype correto (sem newline)
- [ ] Estrutura de diretórios completa

### Output de Validação
```
🔍 VALIDAÇÃO DE ESTRUTURA EPUB
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📖 Validando 94 capítulos...

✅ ESTRUTURA
   ✓ mimetype presente e correto
   ✓ META-INF/container.xml presente
   ✓ content.opf presente

✅ CAPÍTULOS (94/94)
   ✓ Todos os arquivos XHTML válidos
   ✓ Encoding UTF-8 correto
   ✓ Nenhum arquivo vazio
   ✓ Nenhum arquivo truncado

✅ METADADOS
   ✓ Título presente
   ✓ Idioma: pt-BR
   ✓ Capa referenciada

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ Nenhum problema encontrado!
EPUB pronto para empacotamento.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### Checklist de Qualidade Pós-Revisão
- [ ] Itálicos preservados (`<em>` ou `<i>`)
- [ ] Quebras de cena mantidas (✦ ✦ ✦)
- [ ] Aspas tipográficas corretas (" " não " ")
- [ ] Travessões corretos (— não -)
- [ ] Espaçamento consistente
- [ ] Classes CSS aplicadas corretamente
- [ ] Nenhum conteúdo duplicado
- [ ] Ordem dos capítulos correta

