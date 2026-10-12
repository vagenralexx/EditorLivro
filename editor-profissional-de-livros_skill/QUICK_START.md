# ⚡ QUICK START — EDITOR PROFISSIONAL DE LIVROS

**Setup em 3 passos para revisar traduções automaticamente**

---

## 🚀 SETUP RÁPIDO (5 MINUTOS)

### 1️⃣ Copie a skill para seu projeto

```bash
# No terminal, dentro da pasta do seu projeto de tradução:
cp -r /caminho/desta/skill/editor-profissional-de-livros_skill/ .
```

Ou no Windows PowerShell:
```powershell
Copy-Item -Recurse "C:\caminho\da\skill\editor-profissional-de-livros_skill" -Destination .
```

---

### 2️⃣ Organize seus arquivos assim

```
seu-projeto/
  ├── capitulos_extraidos/          # ← Originais em inglês (.txt)
  │   ├── part0001.txt
  │   ├── part0002.txt
  │   └── ...
  │
  ├── traducao_pt_br/               # ← Traduções em PT-BR (.html)
  │   └── epub_build/
  │       └── text/
  │           ├── part0001.html
  │           ├── part0002.html
  │           └── ...
  │
  └── editor-profissional-de-livros_skill/  # ← Skill copiada
      ├── SKILL.md
      ├── tools/
      │   └── corrigir_fluidez_ptbr.py
      └── references/
```

**Importante:** Se suas pastas têm nomes diferentes, ajuste a linha 23 de `tools/corrigir_fluidez_ptbr.py`

---

### 3️⃣ Cole este prompt no chat com o agente

```
Revise esta tradução literária EN→PT-BR de forma COMPLETAMENTE AUTOMÁTICA:

INSTRUÇÕES:
1. Detecte a estrutura do projeto automaticamente
2. Compare TODOS os capítulos traduzidos com os originais em inglês
3. Aplique as regras de fluidez PT-BR (corrija anglicismos)
4. Preserve linguagem crua SEM amenizar (damn→porra, fuck→foder, pussy→buceta)
5. Use o script tools/corrigir_fluidez_ptbr.py se houver mais de 10 arquivos
6. Regenere o EPUB automaticamente ao final
7. NÃO pare entre capítulos — processe tudo de uma vez
8. Entregue apenas um resumo final com estatísticas

ESTRUTURA DO PROJETO:
- Originais EN: capitulos_extraidos/
- Traduções PT-BR: traducao_pt_br/epub_build/text/
- Total de capítulos: [COLOQUE O NÚMERO AQUI]

FOCO ESPECIAL:
- Cenas eróticas: manter intensidade e vocabulário direto
- Voz dos personagens: preservar expressões únicas
- Fluidez: soar natural em PT-BR, não literal do inglês

MODO: AUTOPILOT (sem confirmações intermediárias)
```

---

## ✅ PRONTO!

O agente vai:
1. Detectar sua estrutura
2. Processar todos os capítulos
3. Aplicar 60+ regras de correção
4. Gerar EPUB
5. Entregar resumo em 2-3 minutos

---

## 📚 DOCUMENTAÇÃO COMPLETA

- **SKILL.md** — Documentação completa da skill
- **tools/README.md** — Como usar o script de correção
- **references/automacao.md** — Exemplos de automação
- **PROMPT_REVISAO_AUTOMATICA.md** — Variações do prompt

---

## 🆘 PROBLEMAS?

### "Pasta não encontrada"
→ Ajuste `PASTA_TRADUCAO` em `tools/corrigir_fluidez_ptbr.py` (linha 23)

### "Agente não detectou estrutura"
→ Seja mais explícito no prompt sobre onde estão os arquivos

### "Script não executou"
→ Verifique se Python está instalado: `python --version`

---

## 💡 PRÓXIMOS PASSOS

Após a primeira revisão automática:

1. Abra o EPUB no Calibre ou Kindle Previewer
2. Verifique 2-3 capítulos aleatórios
3. Teste especialmente cenas eróticas
4. Confirme que linguagem crua foi preservada
5. Publique no Kindle! 🎉

---

**Versão:** 1.0  
**Testado em:** "O Abraço de Victoria" (47 capítulos, 2 minutos de processamento)
