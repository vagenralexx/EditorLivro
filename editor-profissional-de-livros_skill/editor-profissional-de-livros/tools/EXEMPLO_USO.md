# 📖 EXEMPLO DE USO — FERRAMENTAS DA SKILL

Este guia mostra como usar as ferramentas da skill em um projeto real.

---

## 🎯 CENÁRIO: Tradução de "Ascensão de Aria"

### Estrutura do projeto:

```
meu-projeto/
  ├── _OceanofPDF.com_Arias_Ascension_-_Stacy_Jones Original.epub
  ├── traducao_capitulos/
  │   ├── part0001.xhtml
  │   ├── part0002.xhtml
  │   ├── ...
  │   └── part0064.xhtml
  └── tools/
      ├── revisar_traducao_completa.py
      └── corrigir_fluidez_ptbr.py
```

---

## 📋 MÉTODO 1: Via Skill (MAIS FÁCIL)

### Passo 1: Abrir projeto no Kiro

```bash
cd meu-projeto
kiro .
```

### Passo 2: Usar comando da skill

No chat do Kiro:

```
/revisar-livro-completo
```

Ou simplesmente:

```
Revise esta tradução EN→PT-BR de forma completa e automática.
Aplique todas as diretrizes da skill.
```

### Resultado:

```
🎯 REVISÃO AUTOMÁTICA COM SKILL DE EDITOR PROFISSIONAL
============================================================
📖 EPUB Original: _OceanofPDF.com_Arias_Ascension_-_Stacy_Jones Original.epub
📂 Traduções: traducao_capitulos/

📖 Extraindo originais...
✅ 64 capítulos extraídos

🔍 Revisando capítulos...
✅ [01/64] part0001.xhtml — 3 correções
✅ [02/64] part0002.xhtml — 5 correções
...

============================================================
🎉 REVISÃO CONCLUÍDA!
============================================================
📚 Capítulos: 64
✅ Corrigidos: 53
🔧 Correções: 194
📈 Taxa: 82.8%

📦 EPUB final: Ascensao_de_Aria_FINAL.epub
📄 Relatório: RELATORIO_REVISAO_SKILL_COMPLETA.md
```

---

## 📋 MÉTODO 2: Script Python Direto

### Passo 1: Copiar ferramentas

```bash
cp -r editor-profissional-de-livros_skill/tools/ meu-projeto/
cd meu-projeto
```

### Passo 2: Executar script

```bash
# Auto-detecta EPUB original e pasta de traduções
python tools/revisar_traducao_completa.py
```

Ou especifique manualmente:

```bash
python tools/revisar_traducao_completa.py \
  "_OceanofPDF.com_Arias_Ascension_-_Stacy_Jones Original.epub" \
  "traducao_capitulos"
```

### Resultado:

Mesmo output do Método 1, mas executado diretamente via Python.

---

## 📋 MÉTODO 3: Apenas Fluidez (Sem Original)

Se você NÃO tem o EPUB original em inglês:

### Passo 1: Ajustar configuração

Edite `tools/corrigir_fluidez_ptbr.py`:

```python
PASTA_TRADUCAO = 'traducao_capitulos'  # Sua pasta aqui
```

### Passo 2: Executar

```bash
python tools/corrigir_fluidez_ptbr.py
```

### Resultado:

```
🔍 CORRETOR DE FLUIDEZ PT-BR
============================================================
📁 Pasta: traducao_capitulos
📄 Arquivos encontrados: 64
🔧 Regras de correção: 60
============================================================

✓ part0001.xhtml: 3 correções
✓ part0002.xhtml: 5 correções
...

============================================================
✅ CONCLUÍDO!
📊 53 arquivos alterados de 64 total
📊 133 correções aplicadas
============================================================
```

---

## 📊 COMPARAÇÃO DOS MÉTODOS

| Método | Facilidade | Velocidade | Precisão | Requer Original EN |
|--------|-----------|-----------|----------|-------------------|
| Via Skill | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ✅ Sim |
| Script Python | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ✅ Sim |
| Apenas Fluidez | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ❌ Não |

---

## 🔍 O QUE CADA MÉTODO FAZ

### `revisar_traducao_completa.py` (Métodos 1 e 2)

✅ **Extração:**
- Extrai capítulos originais EN do EPUB
- Salva em `originais_en/`

✅ **Comparação 1:1:**
- Compara parágrafo a parágrafo
- Detecta amenizações: "meu Deus" em vez de "porra"
- Detecta omissões e adições

✅ **Correções aplicadas:**
- Amenizações: damn→porra, fuck→foder, hell→caralho
- Anglicismos: alcançando→estendendo, enrolando→passando
- Gerúndios: estava caminhando→caminhava
- Conectivos: E então→Então

✅ **Saída:**
- EPUB revisado: `Ascensao_de_Aria_FINAL.epub`
- Relatório: `RELATORIO_REVISAO_SKILL_COMPLETA.md`
- Originais extraídos: `originais_en/` (pode deletar depois)

### `corrigir_fluidez_ptbr.py` (Método 3)

✅ **Correções aplicadas:**
- 60+ padrões de anglicismos
- Gerúndios desnecessários
- Conectivos artificiais

❌ **NÃO faz:**
- Comparação com original EN
- Detecção de amenizações (não sabe o que tinha no original)
- Extração de EPUB

✅ **Saída:**
- Arquivos corrigidos in-place
- Relatório no terminal

---

## 💡 QUANDO USAR CADA MÉTODO

### Use `revisar_traducao_completa.py` quando:
- ✅ Tem acesso ao EPUB original em inglês
- ✅ Quer garantir fidelidade ao original
- ✅ Precisa detectar amenizações não autorizadas
- ✅ Quer relatório detalhado de comparação

### Use `corrigir_fluidez_ptbr.py` quando:
- ✅ NÃO tem acesso ao original
- ✅ Quer apenas melhorar fluidez PT-BR
- ✅ Já revisou fidelidade manualmente
- ✅ Precisa de velocidade máxima

---

## 🎓 EXEMPLO REAL: Correções Aplicadas

### Antes da revisão:

```
"Meu Deus," ela sussurrou, alcançando para ele.
Ele estava caminhando lentamente, enrolando os braços ao redor dela.
E então ela percebeu o que estava acontecendo.
```

### Depois da revisão:

```
"Porra," ela sussurrou, estendendo a mão para ele.
Ele caminhava lentamente, passando os braços ao redor dela.
Então ela percebeu o que acontecia.
```

### Mudanças:
- ✅ "Meu Deus" → "Porra" (amenização corrigida)
- ✅ "alcançando" → "estendendo a mão" (anglicismo)
- ✅ "estava caminhando" → "caminhava" (gerúndio)
- ✅ "enrolando os braços" → "passando os braços" (anglicismo)
- ✅ "E então ela" → "Então ela" (conectivo)
- ✅ "estava acontecendo" → "acontecia" (gerúndio)

---

## 📦 ARQUIVOS GERADOS

### Por `revisar_traducao_completa.py`:

```
meu-projeto/
  ├── Ascensao_de_Aria_FINAL.epub          ← EPUB revisado
  ├── RELATORIO_REVISAO_SKILL_COMPLETA.md  ← Relatório executivo
  ├── originais_en/                         ← Originais extraídos (temporário)
  │   ├── part0001.xhtml
  │   └── ...
  └── traducao_capitulos/                   ← Traduções corrigidas
      ├── part0001.xhtml (modificado)
      └── ...
```

### Por `corrigir_fluidez_ptbr.py`:

```
meu-projeto/
  └── traducao_capitulos/                   ← Traduções corrigidas
      ├── part0001.xhtml (modificado)
      └── ...
```

---

## 🆘 TROUBLESHOOTING

### "Não encontrou EPUB original"

**Problema:** Script não detectou o EPUB automaticamente.

**Solução:**
```bash
# Especifique manualmente
python tools/revisar_traducao_completa.py "Nome_Exato_Do_Arquivo.epub" "traducao_capitulos"
```

### "Pasta de traduções não encontrada"

**Problema:** Script não detectou a pasta automaticamente.

**Solução:**
```bash
# Especifique manualmente
python tools/revisar_traducao_completa.py "Original.epub" "sua_pasta_aqui"
```

### "Erro ao instalar BeautifulSoup"

**Problema:** Dependência não instalada.

**Solução:**
```bash
pip install beautifulsoup4
```

---

**Criado para uso com a skill `editor-profissional-de-livros`**  
**Versão: 1.1 | Data: 28/03/2026**
