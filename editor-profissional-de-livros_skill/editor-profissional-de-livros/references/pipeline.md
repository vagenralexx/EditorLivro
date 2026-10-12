# Pipeline Interno — /revisar e /bloco

## PIPELINE DO /revisar (execute nesta ordem exata)

### ETAPA 1 — LEITURA SILENCIOSA
Leia o texto **sem intervir**. Identifique gênero, POV, tempo verbal, nº de cenas/capítulos, personagens nomeados, tom geral.
Não escreva nada para o usuário nesta etapa.

### ETAPA 2 — DIAGNÓSTICO ESTRUTURAL
Verifique os 5 Mandamentos em cada cena/capítulo:
1. Há um Inciting Incident?
2. As complicações ESCALAM?
3. Há uma crise com opções incompatíveis?
4. O personagem responde ativamente?
5. Há consequência da escolha?

**Value Shift:** Situação muda de positivo→negativo ou vice-versa? Se não → cena MORTA.
Turning points: onde está na estrutura maior? (~25%? ~50%? ~75%?)

### ETAPA 3 — ANTI-IA: SCAN DOS 20 PADRÕES
Para cada padrão encontrado: cite o trecho, identifique o nº (#1–#20), calcule impacto (-0.25 padrão; -0.50 para #11).
Se #11 > 4x em 3K palavras → capítulo reprovado no scan.
Registre: total de padrões, densidade por 1K palavras.

### ETAPA 4 — ANÁLISE DE VOZ
1. **Voice Inhabitation Test:** Habita ou imita o personagem?
2. **Ugly Sentence:** Há ≥1 frase deliberadamente imperfeita?
3. **Voice Under Pressure:** Prosa muda sob estresse?
4. **Cover-the-Name Test:** Leitor identifica quem fala sem ver o nome?

Classifique: PASSA / FALHA / PARCIAL

### ETAPA 5 — ANÁLISE DE PERSONAGENS
Para cada personagem com >3 aparições:
1. Pensamento irrelevante (não conectado ao enredo)?
2. Distorção cognitiva visível?
3. Controle emocional falhando em cena não-climática?
4. Ferida sentida (ambient coloring) ou guardada para reveal?

Classifique: caos presente (X/4) por personagem.

### ETAPA 6 — ANÁLISE DE DIÁLOGOS
1. % de diálogo — está no range do gênero?
2. `FLAT_DIALOGUE` presente?
3. `EXPOSITION DUMP` presente?
4. Tags: overuse de tag única? Tags criativas que chamam atenção?

### ETAPA 7 — ANÁLISE DE HOOK E PULL
1. Primeiras 3-5 frases = 1 dos 6 tipos válidos?
2. Últimas 3-5 frases criam pull?
3. Fechamento resolve emocionalmente demais? (= pull fraco)

Classifique abertura e fechamento: FORTE / MÉDIO / FRACO + tipo.

### ETAPA 8 — GENESIS SCORE
```
Originalidade: X.X — [evidência]
Tema:          X.X — [evidência]
Personagens:   X.X — [evidência, impacto anti-IA descontado]
Prosa & Voz:   X.X — [evidência, impacto anti-IA descontado]
Ritmo:         X.X — [evidência]
Emoção:        X.X — [evidência]
Critério 7:    X.X — [definir pelo gênero]

Floor = nota mínima das 7
Média = soma ÷ 7
GENESIS SCORE = Floor × 0.4 + Média × 0.6 = X.X
```
Threshold: ≥7.5 (literária/memoir) | ≥7.0 (comercial/thriller) | ≥6.5 (NF prescritiva)

### ETAPA 9 — SÍNTESE DO RELATÓRIO

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
RELATÓRIO DE REVISÃO
Gênero: [X] | Gênero inferido: [sim/não]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
◆ GENESIS SCORE: X.X ([APROVADO/REVISÃO NECESSÁRIA])
  Originalidade X.X | Tema X.X | Personagens X.X
  Prosa X.X | Ritmo X.X | Emoção X.X | [7º] X.X

◆ ANTI-IA: [N] padrões ([X.X/1K palavras])
  Range limpo para [gênero]: [N-N]
  ► Críticos: [lista com citação]

◆ ESTRUTURA
  Value Shift: [PRESENTE/AUSENTE]
  5 Mandamentos: [N/5 presentes]
  Cenas mortas: [N] — localizações

◆ VOZ
  Voice Inhabitation: [HABITA/IMITA/PARCIAL]
  Ugly Sentence: [PRESENTE/AUSENTE]
  Voice Under Pressure: [MUDA/ESTÁTICA]
  Cover-the-Name: [PASSA/FALHA]

◆ PERSONAGENS
  Chaos markers: [N/4 por personagem]

◆ DIÁLOGOS
  Proporção: [X%] (alvo: [X-X%])
  Flat dialogue: [N] | Exposition dumps: [N]

◆ HOOK & PULL
  Abertura: [TIPO] — [FORTE/MÉDIO/FRACO]
  Fechamento: [TIPO] — [FORTE/MÉDIO/FRACO]

◆ PRIORIDADES
  🔴 CRÍTICO: [problema mais urgente]
  🟠 IMPORTANTE: [segundo problema]
  🟡 ATENÇÃO: [terceiro problema]
  🟢 PRESERVAR: [o que funciona — não tocar]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### ETAPA 10 — REVISÃO CIRÚRGICA

Taxonomia obrigatória (execute nesta ordem):
1. **ESTRUTURAL** — arco, função do capítulo, ordem de cenas, consistência de comportamento
2. **CONECTIVO** — pontes entre capítulos, transições, progressão emocional
3. **PROSA** — voz, show vs. tell, clichês, padrões anti-IA
4. **FACTUAL** — nomes, datas, lugares, gramática

Regras invioláveis:
- Mudança mínima efetiva
- Preserve a voz do autor
- Não conserte o que funciona
- Não desfaça caos intencional (ugly sentences, pensamentos irrelevantes = features)
- Priorize 🔴 antes de polir prosa

Entregue texto revisado + nota de rodapé com mudanças feitas (1 linha por mudança).

---

## VARIAÇÕES DE MODO

**`/revisar`** — padrão: relatório + texto revisado (etapas 1-10)

**`/revisar modo:sugestoes`** — etapas 1-9 normais; na etapa 10 entrega pares ANTES/DEPOIS:
```
── SUGESTÃO [N] · [🔴/🟠/🟡] · [skill]
PROBLEMA: [1 linha]
ANTES:  "[trecho exato]"
DEPOIS: "[versão proposta]"
POR QUÊ: [justificativa técnica]
──
```
Ao final: "Quais aceita? Todas, só as 🔴, ou liste os números."

**`/revisar modo:relatorio`** — só etapas 1-9, sem reescrever
**`/revisar modo:texto`** — etapas 1-8 silenciosas, só texto revisado
**`/revisar modo:rapido`** — pula etapas 5 e 6; para textos ≤500 palavras
**`/revisar iteracao:N`** — compara com diagnóstico anterior, aponta melhorias e regressões

---

## PIPELINE EXTRA DO /bloco — VERIFICAÇÕES ADICIONAIS

Além das 9 etapas do /revisar, execute para cada bloco:

### CHECK DE CONSISTÊNCIA DE PERSONAGENS
Para cada personagem com aparição no bloco, confronte contra a bíblia de personagens da sessão:

**Testes:**
- O personagem faria isso dado o que sabemos da personalidade dele?
- As reações emocionais são consistentes com o histórico emocional registrado?
- Habilidades/conhecimentos do personagem são consistentes com o que foi estabelecido?
- A voz de diálogo é reconhecível como a mesma do bloco anterior?

**Se inconsistência encontrada:**
```
⚠ [NOME] — INCONSISTÊNCIA DE PERSONAGEM
  O que aconteceu: [citação do texto]
  O que a bíblia diz: [referência à personalidade/histórico]
  Problema: [análise]
  ALTERNATIVA RESPEITOSA: [como chegar no mesmo resultado narrativo
  sem violar a personalidade do personagem]
```

### CHECK DE LINHA DO TEMPO
Construa ou atualize a linha do tempo com cada evento do bloco.
Verifique contra o LOG-RECAP anterior:
- Há anacronia não intencional?
- Durações implícitas são coerentes? (viagem que "leva horas" mas acontece em 3 parágrafos)
- Estações, horários, datas internas batem?

### CHECK DE AMARRAÇÃO ENTRE CAPÍTULOS
Para cada transição de capítulo dentro do bloco:
- O primeiro parágrafo do capítulo novo ancora o leitor no tempo/espaço/personagem correto?
- Há continuidade emocional (ou a quebra é intencional)?
- Algum fio do capítulo anterior ficou pendurado sem sinalização?

### CHECK DE FIOS NARRATIVOS
Cruze com o LOG-RECAP anterior:
- Fios abertos avançaram? (bom)
- Fios abertos foram ignorados completamente? (⚠ alerta — pode ser intencional ou esquecimento)
- Novos fios foram abertos sem setup adequado? (⚠ confusão para o leitor)

### CHECK DE SUBDESENVOLVIMENTO DE PERSONAGENS
Sinais de alerta a identificar ativamente:
- Personagem secundário importante que não tem vida interna própria (só existe para servir ao protagonista)
- Personagem que muda de opinião sem motivação interna visível
- Personagem cuja única função é expor informação ou fazer perguntas
- Protagonista que não erra, não duvida, ou cujas falhas nunca têm consequências reais

---

## REVISÃO CIRÚRGICA — TAXONOMIA COMPLETA

### Prioridade 1: ESTRUTURAL (Risco: ALTO)
Arco não avança / capítulo não serve à função / cenas em ordem errada / comportamento inconsistente / capítulo redundante

### Prioridade 2: CONECTIVO (Risco: MÉDIO)
Abertura sem ponte do anterior / fechamento sem apontar para o próximo / transições abruptas / progressão emocional que pula etapas

### Prioridade 3: PROSA (Risco: BAIXO)
Deriva de voz / show vs. tell / clichês / padrões anti-IA / metáforas fracas / monotonia de frase / palavras de difícil leitura sem propósito / jargão que quebra imersão

### Prioridade 4: FACTUAL/PONTUAL (Risco: ZERO)
Nomes, datas, lugares / inconsistências físicas / gramática / ortografia
