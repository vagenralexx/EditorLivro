# AUTOMAÇÃO E DETECÇÃO INTELIGENTE

## OBJETIVO
Reduzir fricção e comandos manuais. O agente deve entender o contexto e agir autonomamente.

---

## FLUXO IDEAL DE TRABALHO

### CENÁRIO 1: Revisão de Tradução Completa (como "O Abraço de Victoria")

**Usuário diz:**
> "Revise a tradução deste livro, compare com o original em inglês, mantenha linguagem crua e corrija fluidez em PT-BR"

**Agente detecta automaticamente:**
1. ✓ Estrutura de projeto: `capitulos_extraidos/` (EN) + `traducao_pt_br/epub_build/text/` (PT-BR)
2. ✓ 47 pares de arquivos numerados sequencialmente
3. ✓ É tradução EN→PT-BR
4. ✓ Usuário quer revisão completa (não bloco a bloco)

**Agente executa SEM PERGUNTAR:**
1. Cria script Python com:
   - Tabela de equivalências (damn→porra, fuck→foder, etc.)
   - Lista de 22 anglicismos (alcançando→estendendo, nuzzlar→roçar, etc.)
   - Correções de cenas eróticas (pussy→buceta, cock→pau)
2. Executa script em todos os 47 arquivos
3. Reporta estatísticas: "54 correções em 20 arquivos"
4. Regenera EPUB automaticamente
5. Entrega resumo final

**Tempo total:** 2-3 minutos (vs 2-3 horas fazendo capítulo por capítulo)

---

### CENÁRIO 2: Continuação de Revisão

**Usuário diz:**
> "Continue a revisão do capítulo 19 em diante sem parar"

**Agente detecta automaticamente:**
1. ✓ Há LOG-RECAP dos capítulos 1-18 no contexto
2. ✓ Próximo capítulo é o 19
3. ✓ Usuário quer modo contínuo ("sem parar")

**Agente executa SEM PERGUNTAR:**
1. Carrega cap. 19 (original EN + tradução PT-BR)
2. Aplica todas as regras
3. Salva correções
4. Avança para cap. 20 automaticamente
5. Repete até o último capítulo
6. Gera LOG-RECAP final consolidado

**Não faz:**
- ❌ Parar entre capítulos para pedir confirmação
- ❌ Perguntar "devo continuar?"
- ❌ Listar cada correção individualmente

---

### CENÁRIO 3: Correção de Fluidez em Livro Já Traduzido

**Usuário diz:**
> "Aplique as regras de fluidez PT-BR em todo o livro"

**Agente detecta automaticamente:**
1. ✓ Livro já está traduzido
2. ✓ Não precisa comparar com original
3. ✓ Foco apenas em anglicismos e fluidez

**Agente executa SEM PERGUNTAR:**
1. Cria script Python focado apenas em anglicismos
2. Executa em todos os arquivos
3. Regenera EPUB
4. Reporta estatísticas

---

## SINAIS DE INTENÇÃO DO USUÁRIO

### Sinais de "MODO AUTOPILOT" (não parar, não perguntar)
- "continue sem parar"
- "faça tudo de uma vez"
- "revise o livro completo"
- "até o final"
- "não precisa perguntar"
- "automaticamente"
- "em todos os capítulos"

### Sinais de "MODO COLABORATIVO" (pedir confirmação)
- "me mostre as sugestões"
- "o que você mudaria?"
- "preciso aprovar"
- "modo:sugestoes"

### Sinais de "MODO BLOCO A BLOCO" (pausar entre blocos)
- "revise o capítulo X"
- "vamos por partes"
- "um bloco de cada vez"
- Nenhuma menção a "completo" ou "tudo"

---

## SCRIPT PYTHON — TEMPLATE REUTILIZÁVEL

Quando criar scripts de automação, use este template:

```python
#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Script de revisão automática
Gerado pela skill editor-profissional-de-livros
"""

import os
import re
from pathlib import Path

# CORREÇÕES DE LINGUAGEM CRUA (EN→PT-BR)
LINGUAGEM_CRUA = [
    (r'\bdamn\b', 'porra'),
    (r'\bgoddamn\b', 'caralho'),
    (r'\bhell\b', 'porra'),
    (r'\bfucking\b', 'fodido'),
    # ... adicionar todas da tabela
]

# CORREÇÕES DE ANGLICISMOS
ANGLICISMOS = [
    (r'\bAlcançando\b', 'Estendendo a mão'),
    (r'\benrolando os braços\b', 'passando os braços ao redor'),
    # ... adicionar todas as 22 regras
]

# CORREÇÕES DE CENAS ERÓTICAS
CENAS_EROTICAS = [
    (r'\bo sexo (dela|dele)\b', r'a buceta \1'),
    (r'\bpossuí-la\b', 'fodê-la'),
    # ... adicionar todas
]

def corrigir_arquivo(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        conteudo = f.read()
    
    original = conteudo
    
    # Aplicar todas as correções
    for padrao, sub in LINGUAGEM_CRUA + ANGLICISMOS + CENAS_EROTICAS:
        conteudo = re.sub(padrao, sub, conteudo)
    
    if conteudo != original:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(conteudo)
        return True
    return False

def main():
    pasta = Path('traducao_pt_br/epub_build/text')
    arquivos = sorted(pasta.glob('*.html'))
    
    alterados = 0
    for arquivo in arquivos:
        if corrigir_arquivo(arquivo):
            alterados += 1
            print(f"✓ {arquivo.name}")
    
    print(f"\n✅ {alterados} arquivos corrigidos")

if __name__ == '__main__':
    main()
```

---

## CHECKLIST DE AUTOMAÇÃO

Antes de pedir confirmação ao usuário, verifique:

- [ ] A estrutura do projeto é clara?
- [ ] O usuário deu sinais de "modo autopilot"?
- [ ] As regras a aplicar são mecânicas (não requerem julgamento)?
- [ ] Há mais de 5 arquivos para processar?

**Se SIM para todos:** → Criar script e executar automaticamente  
**Se NÃO para algum:** → Processar manualmente ou pedir clarificação

---

## EXEMPLOS DE BOA DETECÇÃO

### ✅ BOM — Agente age autonomamente
**Usuário:** "revise a tradução comparando com o original, sem amenizar"  
**Agente:** [detecta estrutura] → [cria script] → [executa] → [reporta]

### ✅ BOM — Agente entende continuação
**Usuário:** "continue"  
**Agente:** [identifica próximo capítulo] → [processa] → [avança]

### ✅ BOM — Agente detecta cena erótica
**Agente lê:** "He thrust into her pussy..."  
**Agente:** [ativa regras de cenas eróticas automaticamente] → [aplica]

### ❌ RUIM — Agente pergunta demais
**Usuário:** "revise o livro"  
**Agente:** "Você quer que eu revise capítulo por capítulo ou tudo de uma vez?"  
**Problema:** Contexto já indica (47 arquivos, estrutura clara)

### ❌ RUIM — Agente não detecta padrão
**Agente lê:** "Holy shit" traduzido como "Meu Deus"  
**Agente:** [lista como sugestão em vez de corrigir]  
**Problema:** Tabela de equivalências é obrigatória, não opcional

---

## MELHORIAS FUTURAS (V2.0)

1. **Detecção de voz de personagem:**
   - Criar glossário automático de expressões únicas por personagem
   - Alertar se personagem usar expressão de outro personagem

2. **Aprendizado de padrões:**
   - Salvar correções aplicadas em arquivo `.corrections.json`
   - Reutilizar padrões em projetos futuros do mesmo autor

3. **Validação automática:**
   - Após aplicar correções, fazer segunda passada para verificar
   - Detectar se criou novos problemas (ex: "a a buceta" por substituição errada)

4. **Integração com EPUB:**
   - Detectar quando EPUB precisa ser regenerado
   - Fazer automaticamente sem perguntar

5. **Modo "diff" visual:**
   - Gerar arquivo HTML mostrando antes/depois das correções
   - Útil para revisão humana final
