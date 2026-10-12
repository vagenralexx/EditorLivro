# FERRAMENTAS — EDITOR PROFISSIONAL DE LIVROS

Esta pasta contém scripts Python prontos para uso em projetos de tradução literária.

---

## 🚀 INÍCIO RÁPIDO

### Para revisão completa com original EN (RECOMENDADO):
```bash
# 1. Tenha seu EPUB original e pasta de traduções prontos
# 2. Execute (auto-detecta tudo):
python tools/revisar_traducao_completa.py

# Ou especifique manualmente:
python tools/revisar_traducao_completa.py "Original.epub" "traducao_capitulos"
```

### Para correção de fluidez apenas (sem original):
```bash
python tools/corrigir_fluidez_ptbr.py
```

### Via skill (mais fácil):
```
/revisar-livro-completo
```

---

## 📦 FERRAMENTAS DISPONÍVEIS

### 1. `revisar_traducao_completa.py` ⭐ RECOMENDADO

**O que faz:**
- Revisão 1:1 completa comparando original EN com tradução PT-BR
- Aplica TODAS as diretrizes da skill automaticamente
- Detecta e corrige amenizações não autorizadas
- Corrige anglicismos, gerúndios e conectivos
- Preserva linguagem crua em cenas eróticas
- Regenera EPUB automaticamente
- Gera relatório detalhado

**Quando usar:**
- Para revisão completa de tradução EN→PT-BR
- Quando precisa garantir fidelidade ao original
- Para aplicar tabela de equivalências obrigatórias (damn→porra, fuck→foder)
- Modo autopilot: processa tudo sem parar

**Como usar:**
```bash
# Auto-detecta EPUB original e pasta de traduções
python tools/revisar_traducao_completa.py

# Ou especifique manualmente
python tools/revisar_traducao_completa.py "Original.epub" "traducao_capitulos"
```

**Estrutura esperada:**
```
seu-projeto/
  ├── Original.epub (ou similar com "Original" no nome)
  ├── traducao_capitulos/
  │   ├── part0001.xhtml
  │   ├── part0002.xhtml
  │   └── ...
  └── tools/
      └── revisar_traducao_completa.py
```

**Saída esperada:**
```
🎯 REVISÃO AUTOMÁTICA COM SKILL DE EDITOR PROFISSIONAL
============================================================
📖 EPUB Original: Original.epub
📂 Traduções: traducao_capitulos/

📖 Extraindo originais de: Original.epub
✅ 64 capítulos extraídos

🔍 Revisando capítulos...
✅ [01/64] part0001.xhtml — 3 correções
✅ [02/64] part0002.xhtml — 5 correções
...

🔄 Regenerando EPUB...
✅ EPUB gerado: Ascensao_de_Aria_FINAL.epub

============================================================
🎉 REVISÃO CONCLUÍDA COM SUCESSO!
============================================================
📚 Capítulos: 64
✅ Corrigidos: 53
🔧 Correções: 194
📈 Taxa: 82.8%

📦 EPUB final: Ascensao_de_Aria_FINAL.epub
📄 Relatório: RELATORIO_REVISAO_SKILL_COMPLETA.md
```

**Correções aplicadas:**
- ✅ Amenizações: damn→porra, fuck→foder, hell→caralho
- ✅ Anglicismos: alcançando→estendendo, enrolando→passando
- ✅ Gerúndios: estava caminhando→caminhava
- ✅ Conectivos: E então→Então
- ✅ Preservação de linguagem crua integral

---

### 2. `corrigir_fluidez_ptbr.py`

**O que faz:**
- Corrige 60+ padrões de anglicismos em traduções PT-BR
- Melhora fluidez natural do português brasileiro
- Preserva linguagem crua em cenas eróticas
- Processa múltiplos arquivos automaticamente

**Quando usar:**
- Após traduzir um livro do inglês para PT-BR
- Quando detectar construções como "alcançando", "nuzzlar", "enrolando braços"
- Para garantir que tradução soa natural em PT-BR

**Como usar:**
```bash
# 1. Copie o arquivo para a raiz do seu projeto
cp corrigir_fluidez_ptbr.py /caminho/do/seu/projeto/

# 2. Ajuste a variável PASTA_TRADUCAO se necessário
# (padrão: traducao_pt_br/epub_build/text)

# 3. Execute
python corrigir_fluidez_ptbr.py
```

**Estrutura esperada do projeto:**
```
seu-projeto/
  ├── traducao_pt_br/
  │   └── epub_build/
  │       └── text/
  │           ├── part0001.html
  │           ├── part0002.html
  │           └── ...
  └── corrigir_fluidez_ptbr.py
```

**Saída esperada:**
```
🔍 CORRETOR DE FLUIDEZ PT-BR
============================================================
📁 Pasta: traducao_pt_br/epub_build/text
📄 Arquivos encontrados: 47
🔧 Regras de correção: 60
============================================================

✓ part0004.html: 3 correções
✓ part0009.html: 5 correções
✓ part0012.html: 4 correções
...

============================================================
✅ CONCLUÍDO!
📊 20 arquivos alterados de 47 total
📊 54 correções aplicadas
============================================================
```

---

## 🔧 PERSONALIZAÇÃO

### Ajustar pasta de tradução

Edite a linha 23 do script:

```python
PASTA_TRADUCAO = 'traducao_pt_br/epub_build/text'  # Sua pasta aqui
```

Exemplos:
- `'traducao/capitulos'`
- `'output/html'`
- `'epub/text'`

### Adicionar novas regras

Edite as listas de correções (linhas 30-120):

```python
ANGLICISMOS_MOVIMENTO = [
    (r'\bseu_padrao\b', 'sua_correcao'),
    # Adicione mais aqui
]
```

---

## 📋 REGRAS APLICADAS

### Categorias de correção:

1. **Anglicismos de Movimento (21 regras)**
   - "alcançando" → "estendendo a mão"
   - "enrolando os braços" → "passando os braços ao redor"
   - "enganou as coxas" → "colocou as coxas"
   - "a cavaleiro" → "montada"

2. **Anglicismos de Expressão (12 regras)**
   - "arrastou as palavras" → "disse arrastado"
   - "nuzzlar" → "roçar o rosto"
   - "clareou a garganta" → "pigarreou"
   - "arqueou sobrancelha" → "ergueu sobrancelha"

3. **Anglicismos de Percepção (6 regras)**
   - "forçando pálpebras a abrirem" → "abrindo olhos com esforço"
   - "inclinando cabeça para trás" → "erguendo cabeça"

4. **Construções Eróticas Literais (10 regras)**
   - "empurrões" → "estocadas"
   - "bombeando-a cheia" → "enchendo-a com"
   - "se enrolava apertado" → "crescia intenso"

5. **Outras Melhorias (11 regras)**
   - "cambaleando nos pés" → "cambaleando"
   - "soltou um bafo" → "soltou o ar"
   - "galvanizados" → "animados"

---

## ⚠️ IMPORTANTE

### O script NÃO altera:
- ❌ Linguagem crua (porra, caralho, foder, buceta)
- ❌ Nomes de personagens
- ❌ Termos técnicos ou nomes próprios
- ❌ Estrutura HTML dos arquivos

### O script APENAS corrige:
- ✅ Anglicismos estruturais
- ✅ Construções não naturais em PT-BR
- ✅ Redundâncias

---

## 🚀 INTEGRAÇÃO COM WORKFLOW

### Workflow recomendado:

```
1. Traduzir livro (manual ou IA)
   ↓
2. Executar revisar_traducao_completa.py (se tiver original EN)
   OU corrigir_fluidez_ptbr.py (se não tiver original)
   ↓
3. Revisar manualmente cenas críticas
   ↓
4. Gerar EPUB
   ↓
5. Testar no Kindle Previewer
```

### Uso com a skill:

Quando usar a skill `editor-profissional-de-livros`, você pode:

**Opção 1 — Comando da skill (RECOMENDADO):**
```
Usuário: "/revisar-livro-completo"
Agente: [detecta estrutura] → [usa tools/revisar_traducao_completa.py] → [reporta]
```

**Opção 2 — Prompt natural:**
```
Usuário: "Revise esta tradução EN→PT-BR de forma completa e automática"
Agente: [detecta contexto] → [usa tools/revisar_traducao_completa.py] → [reporta]
```

**Opção 3 — Script direto:**
```bash
python editor-profissional-de-livros_skill/tools/revisar_traducao_completa.py
```

**Opção 4 — Apenas fluidez (sem original EN):**
```bash
python editor-profissional-de-livros_skill/tools/corrigir_fluidez_ptbr.py
```

### Comparação das ferramentas:

| Ferramenta | Precisa Original EN? | O que faz | Quando usar |
|------------|---------------------|-----------|-------------|
| `revisar_traducao_completa.py` | ✅ Sim | Revisão 1:1 completa + correções | Revisão completa com fidelidade |
| `corrigir_fluidez_ptbr.py` | ❌ Não | Apenas correções de fluidez | Pós-processamento rápido |

---

## 📚 REFERÊNCIAS

Baseado em análise de:
- 15+ bestsellers (90M+ cópias vendidas)
- Padrões identificados em "O Abraço de Victoria" (Stacy Jones)
- Diretrizes de tradução literária PT-BR
- Skill `editor-profissional-de-livros` V1.1

---

## 🆘 SUPORTE

**Problemas comuns:**

### "Pasta não encontrada"
- Verifique se `PASTA_TRADUCAO` está correta
- Use caminho relativo a partir de onde executa o script

### "Nenhum arquivo .html encontrado"
- Confirme que arquivos têm extensão `.html`
- Verifique se está na pasta correta

### "Erro ao ler/escrever arquivo"
- Verifique permissões de leitura/escrita
- Confirme encoding UTF-8 dos arquivos

---

## 📝 CHANGELOG

### V1.1 (28/03/2026)
- ➕ Adicionado `revisar_traducao_completa.py` — revisão 1:1 completa com original EN
- ✅ Comparação parágrafo a parágrafo
- ✅ Detecção de amenizações não autorizadas
- ✅ Aplicação de tabela de equivalências obrigatórias
- ✅ Extração automática de originais do EPUB
- ✅ Regeneração automática de EPUB
- ✅ Relatório detalhado em Markdown

### V1.0 (28/03/2026)
- Versão inicial
- 60 regras de correção
- Suporte para múltiplos arquivos
- Relatório detalhado de alterações
