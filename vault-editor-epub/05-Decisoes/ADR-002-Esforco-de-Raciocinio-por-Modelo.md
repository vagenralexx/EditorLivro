# ADR-002 — Seletor de esforço de raciocínio por modelo

**Status:** implementado  
**Data:** 2026-10-11

## Contexto

O fluxo editorial anterior expirava aos 90 segundos em modelos de raciocínio porque o orçamento de `max_tokens` era pequeno demais e o modelo gastava toda a conclusão em raciocínio, sem entregar o JSON final. Um teste real com texto fictício no `z-ai/glm-5.3-flash` confirmou que `reasoning_effort: "low"`, `chat_template_kwargs: { "clear_thinking": true }` e `max_tokens: 4096` retornam `200 OK` com análise válida em ~34s. Esse resultado foi verificado apenas para esse modelo.

Ao mesmo tempo, cada família documenta nomes e níveis diferentes (`reasoning_effort` com `low/high/max` na GLM; `enable_thinking` nos Nemotron 3; `reasoning_effort` com `low/medium/high` na OpenAI; ausência no Groq). Enviar o mesmo parâmetro a todos os provedores causaria erros 400 ou comportamento incorreto.

## Decisão

1. Adicionar o campo **Esforço de raciocínio** ao painel do assistente editorial, junto ao seletor de modelo.
2. Oferecer apenas os níveis documentados para o modelo selecionado — nunca converter um nível não suportado silenciosamente.
3. Usar **Baixo** como padrão para revisão de página.
4. Persistir a escolha por provedor e modelo no `localStorage` (mesma chave de preferências do catálogo e do modelo), sem exigir nova validação da chave ao trocar o esforço.
5. Quando o modelo não permitir controlar o esforço, mostrar **Automático** com uma explicação curta.
6. Extrair capacidades e parâmetros para o módulo `apps/desktop/src-tauri/src/model_capabilities.rs`, expondo:
   - `effort_options(provider, model)` — níveis, padrão e nota explicativa (usado pelo comando `get_model_effort_options`).
   - `apply_model_parameters(body, provider, model, effort)` — traduz a escolha em parâmetros de requisição e rejeita níveis inválidos.

## Escopo de capacidade implementado (verificado/documentado)

| Família | Níveis oferecidos | Parâmetros enviados | Estado |
|---|---|---|---|
| NVIDIA `z-ai/glm-*` (GLM 5.x) | Baixo / Alto / Máximo | `reasoning_effort`, `chat_template_kwargs.clear_thinking=true`, `max_tokens` 4096 (`low`) / 8192 (`high`/`max`) | `low` verificado ao vivo; `high`/`max` documentados, não verificados ao vivo |
| NVIDIA `nemotron-3-super-*` / `nemotron-3-ultra-*` | Automático (não configurável) | `chat_template_kwargs.enable_thinking=false`, `temperature=1.0`, `top_p=0.95` | configuração preservada da versão anterior |
| Groq, OpenAI e demais NVIDIA | Automático (não configurável) | nenhum parâmetro de esforço | intencionalmente fora desta entrega |

## Limitações conhecidas

- OpenAI série `o1`/`o3`/`o4`/`gpt-5` documenta `reasoning_effort` (`low`/`medium`/`high`), e o Groq rejeita `reasoning_effort` em modelos sem raciocínio. Ambos ficaram como **Automático** porque a camada comum ainda envia `temperature`/`top_p`/`max_tokens` (incompatíveis com os modelos de raciocínio da OpenAI) e nenhuma chamada dessas famílias foi testada ao vivo. Ativar exigirá um teste real e tratamento próprio de parâmetros.
- `max_tokens` para `high`/`max` na GLM foi estimado em 8192 para dar margem ao raciocínio; apenas `low` (4096) foi verificado ao vivo.
- `nvidia_live_diagnostic` continua opt-in e ignorado pela suíte padrão.

## Consequências

- `lib.rs` deixa de acumular regras por modelo; novas famílias entram no módulo de capacidades.
- A UI passa a declarar explicitamente quando um modelo não controla o esforço, em vez de enviar parâmetros presumidos.
- A saída editorial estruturada em pt-BR (resumo, achados com severidade, sugestões Antes/Depois com justificativa) permanece inalterada; `reasoning_content` nunca é exposto como análise final.
