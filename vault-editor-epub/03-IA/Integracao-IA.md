# Integração de IA editorial

## Posição do produto

A IA é uma camada de assistência editorial, não a fonte de verdade e não uma ferramenta de substituição silenciosa. Ela recebe somente o contexto escolhido pela pessoa usuária e retorna análises/sugestões auditáveis.

## Provedores

Definir uma interface comum com adaptadores para:

- APIs OpenAI-compatible, incluindo Groq quando o endpoint/modelo escolhido seguir esse formato.
- OpenAI/GPT.
- Outros provedores futuros sem alterar o domínio editorial.

### NVIDIA NIM

NVIDIA NIM é um provedor prioritário. O adaptador usará o endpoint OpenAI-compatible `https://integrate.api.nvidia.com/v1/chat/completions`, permitindo ao usuário escolher os modelos publicados no catálogo no momento da configuração. A NVIDIA oferece acesso de desenvolvimento/POC para membros do Developer Program, mas disponibilidade, limites e condições dependem do modelo e da conta; o app jamais deve apresentar isso como uso ilimitado garantido. O mesmo contrato também permite configurar um NIM auto-hospedado mais adiante.

Configuração por provedor: nome, endpoint, modelo, parâmetros permitidos, limite de contexto, custo estimado quando fornecido, e política de retenção/link de privacidade. A chave fica no keychain/secure store, nunca no EPUB, backup ou repositório.

## Ordem obrigatória da configuração

1. A pessoa escolhe o provedor e salva sua chave no cofre do sistema.
2. O aplicativo consulta `GET /v1/models` do endpoint OpenAI-compatível do provedor com essa chave.
3. Somente se a consulta retornar sucesso, a UI libera o seletor com os IDs de modelo efetivamente disponíveis para aquela conta.
4. A análise usa exclusivamente um modelo escolhido dessa lista.

Não pré-selecionar um ID apenas porque ele já existiu na documentação. Catálogos e disponibilidades mudam; um `410 Gone` deve ser mostrado como modelo/recurso descontinuado e resolvido carregando o catálogo atual, não repetindo a mesma chamada de chat.

## Contexto enviado

O painel oferece escolhas explícitas: seleção, parágrafo, capítulo aberto, capítulos vizinhos, bíblia do livro, glossário e relatório anterior. Antes da primeira chamada de cada escopo, informar quantidade estimada de texto e provedor de destino.

Para capítulos longos: dividir em blocos com recap estruturado (personagens, linha do tempo, fios e decisões), em vez de enviar o livro inteiro repetidamente.

## Modos de IA derivados da skill editorial

| Modo | Resultado no app | Origem conceitual |
|---|---|---|
| Revisão de linha | sugestões por trecho com severidade | revisão cirúrgica |
| Diagnóstico de capítulo | relatório de estrutura, voz, personagens, diálogo, hook e prioridades | pipeline de revisão |
| Análise sistêmica | mapa de fios, coerência, ritmo e arcos entre capítulos | revisão sistêmica |
| Genesis Score | nota explicada por dimensão e evidências | scores |
| Revisão de tradução | pares original/tradução, omissões, tom e glossário | revisão 1:1 |
| Fluidez PT-BR | candidatos a anglicismos/gerúndios/conectivos, aprováveis em lote | automação |
| Voz e consistência | alertas contra bíblia de personagens/glossário | análise modular |
| Brainstorm | alternativas de cena, conflito, arco, sinopse ou hook; separadas do texto | criação |

## Como transformar a skill em produto

Não convém carregar um `SKILL.md` inteiro como prompt opaco. Extrair seu conteúdo em pacotes versionados:

- `editorial-rules`: taxonomia, checklist e formato de saída.
- `genre-profile`: pesos, metas e linguagem permitida por gênero.
- `project-bible`: personagens, termos, POV, tempo verbal, tom e decisões do livro.
- `analysis-template`: contrato JSON/estruturado por modo.

Cada resposta deve conter `findings[]`, `suggestions[]`, citações/âncoras, confiança, impacto e justificativa. A interface renderiza um diff; aplicar uma sugestão cria um `ChangeSet` reversível. Regras mecânicas em massa também mostram contagem, amostra e backup antes da execução.

## Parâmetros e diagnóstico de inferência

Os parâmetros de raciocínio são definidos por modelo, usando capacidades documentadas. Reduzir apenas `max_tokens` pode impedir que um modelo de raciocínio entregue sua resposta final.

O painel do assistente editorial oferece um campo **Esforço de raciocínio** ao lado do seletor de modelo. As capacidades e os parâmetros de cada família vivem no módulo próprio `apps/desktop/src-tauri/src/model_capabilities.rs` (fonte única de verdade), não em `lib.rs`. A escolha é persistida por provedor e modelo no `localStorage` do app, junto com o catálogo e o modelo selecionado, e vale na próxima análise sem revalidar a chave.

Regras aplicadas pelo módulo:

- Só envia um parâmetro quando a documentação oficial da família o descreve; não presume nomes, níveis ou valores iguais entre famílias.
- Se a UI receber um nível não suportado, o backend retorna erro explícito em vez de converter silenciosamente.
- O padrão para revisão de página é sempre o esforço mais baixo disponível.

Capacidades por família:

- `z-ai/glm-*` (GLM 5.x, ex.: `z-ai/glm-5.3-flash`): níveis `low`, `high` e `max` (padrão `max`), expostos como **Baixo / Alto / Máximo**. O nível "médio" **não** é oferecido porque a documentação não o lista. Envia `reasoning_effort` (nível escolhido), `chat_template_kwargs.clear_thinking: true` (Z.AI pede `true` para cenários de chat) e orçamento de conclusão `4096` tokens para `low` (verificado ao vivo) e `8192` para `high`/`max` (margem maior para a cadeia de raciocínio, ainda não verificada ao vivo). A resposta editorial continua limitada a três achados e três sugestões.
- `nvidia/nemotron-3-super-*` / `nvidia/nemotron-3-ultra-*`: **não configurável** por níveis — controla o modo de pensamento via `chat_template_kwargs.enable_thinking`. Para revisão de página o app usa `enable_thinking: false` com a amostragem recomendada (`temperature: 1.0`, `top_p: 0.95`).
- Demais modelos (Groq, OpenAI, outros NVIDIA): **Automático / não configurável**, mantêm os parâmetros comuns e não recebem `reasoning_effort` nem `chat_template_kwargs`.
- OpenAI (série `o1`/`o3`/`o4`/`gpt-5`) documenta `reasoning_effort` com `low`/`medium`/`high`, e o Groq rejeita `reasoning_effort` em modelos sem raciocínio. Ambos foram **deixados como "Automático"** nesta entrega: a camada comum ainda envia `temperature`/`top_p`/`max_tokens`, que conflitam com os modelos de raciocínio da OpenAI, e nenhuma chamada foi verificada ao vivo para essas famílias. Ativar esforço neles exige um teste real e um tratamento próprio de parâmetros.

- O aplicativo registra somente provedor, modelo, tamanho da entrada, duração, status HTTP, motivo de término e contagens de conteúdo. Não registrar chave, manuscrito ou raciocínio.
- O teste de integração `nvidia_live_diagnostic` é opt-in e usa somente texto fictício. Em 11/10/2026, o fluxo anterior de revisão expirou aos 90s; o fluxo com esforço baixo retornou `200 OK`, resumo, três achados e três sugestões em aproximadamente 34s no GLM 5.3 Flash. Esse resultado não garante a disponibilidade de todos os modelos nem a mesma duração para páginas maiores.

Referências: https://huggingface.co/zai-org/GLM-5.3-Flash · https://learn.microsoft.com/azure/ai-foundry/openai/how-to/reasoning

## Proteções necessárias

- Nunca enviar texto ou chamar IA sem chave, provedor e consentimento configurados.
- Avisar que provedores externos processam o conteúdo enviado; oferecer modo local/desativado.
- Limitar chamadas, cancelar streaming e registrar apenas metadados locais por padrão.
- Tratar saída do modelo como conteúdo não confiável: não executar instruções nem código retornado.
- A revisão de tradução não deve aplicar substituições globais cegas: contexto, personagem e gênero podem mudar a melhor equivalência.

