// Os ids abaixo precisam existir em src-tauri/src/providers.rs (que guarda os endpoints).
export const providerIds = ["nvidia", "groq", "openai", "openrouter", "gemini", "mistral", "deepseek", "xai", "together", "cerebras"] as const;
export type ProviderId = (typeof providerIds)[number];

export interface ProviderOption {
  id: ProviderId;
  label: string;
  badge: string;
  endpoint: string;
  /** Texto de ajuda sobre onde a chave é emitida. */
  keyHint: string;
  /** Prefixos que identificam a chave sem ambiguidade (usados para detectar o servidor ao colar). */
  keyPrefixes: string[];
}

export const providers: ProviderOption[] = [
  { id: "nvidia", label: "NVIDIA NIM", badge: "NIM", endpoint: "integrate.api.nvidia.com", keyHint: "Chave começa com nvapi-", keyPrefixes: ["nvapi-"] },
  { id: "groq", label: "Groq", badge: "Groq", endpoint: "api.groq.com", keyHint: "Chave começa com gsk_", keyPrefixes: ["gsk_"] },
  { id: "openai", label: "OpenAI", badge: "GPT", endpoint: "api.openai.com", keyHint: "Chave começa com sk-proj- ou sk-", keyPrefixes: ["sk-proj-", "sk-svcacct-", "sk-admin-"] },
  { id: "openrouter", label: "OpenRouter", badge: "OR", endpoint: "openrouter.ai", keyHint: "Chave começa com sk-or-; dá acesso a modelos de vários fabricantes", keyPrefixes: ["sk-or-"] },
  { id: "gemini", label: "Google Gemini", badge: "Gemini", endpoint: "generativelanguage.googleapis.com", keyHint: "Chave do Google AI Studio, começa com AIza", keyPrefixes: ["AIza"] },
  { id: "mistral", label: "Mistral AI", badge: "Mistral", endpoint: "api.mistral.ai", keyHint: "Chave de 32 caracteres, sem prefixo", keyPrefixes: [] },
  { id: "deepseek", label: "DeepSeek", badge: "DS", endpoint: "api.deepseek.com", keyHint: "Chave começa com sk- (32 caracteres hexadecimais)", keyPrefixes: [] },
  { id: "xai", label: "xAI (Grok)", badge: "xAI", endpoint: "api.x.ai", keyHint: "Chave começa com xai-", keyPrefixes: ["xai-"] },
  { id: "together", label: "Together AI", badge: "TG", endpoint: "api.together.xyz", keyHint: "Chave de 64 caracteres hexadecimais", keyPrefixes: [] },
  { id: "cerebras", label: "Cerebras", badge: "CS", endpoint: "api.cerebras.ai", keyHint: "Chave começa com csk-", keyPrefixes: ["csk-"] },
];

export function providerFor(id: ProviderId): ProviderOption {
  return providers.find((provider) => provider.id === id) ?? providers[0];
}

/**
 * Tenta reconhecer o servidor pelo formato da chave. Devolve `null` quando o
 * formato é ambíguo ou desconhecido (nesse caso mantemos o provedor escolhido).
 */
export function detectProviderFromKey(rawKey: string): ProviderId | null {
  const key = rawKey.trim();
  if (key.length < 8) return null;
  for (const provider of providers) {
    if (provider.keyPrefixes.some((prefix) => key.startsWith(prefix))) return provider.id;
  }
  if (/^sk-[0-9a-f]{32}$/u.test(key)) return "deepseek";
  if (/^[0-9a-f]{64}$/u.test(key)) return "together";
  if (/^sk-[A-Za-z0-9_-]{40,}$/u.test(key)) return "openai";
  return null;
}

export function emptyRecord<T>(make: () => T): Record<ProviderId, T> {
  return Object.fromEntries(providerIds.map((id) => [id, make()])) as Record<ProviderId, T>;
}

const organisationNames: Record<string, string> = {
  "01-ai": "01.AI", "ai21": "AI21", "deepseek-ai": "DeepSeek", "google": "Google", "meta": "Meta", "microsoft": "Microsoft", "mistralai": "Mistral AI", "moonshotai": "Moonshot AI", "nvidia": "NVIDIA", "openai": "OpenAI", "qwen": "Qwen", "z-ai": "Z.AI", "anthropic": "Anthropic", "x-ai": "xAI", "cohere": "Cohere", "perplexity": "Perplexity", "nousresearch": "Nous Research", "meta-llama": "Meta", "amazon": "Amazon", "mistral": "Mistral AI", "ibm": "IBM",
};

/** A readable label while preserving the exact provider id as secondary text. */
export function friendlyModelName(modelId: string) {
  const [organisation, ...modelParts] = modelId.split("/");
  const model = (modelParts.join("/") || organisation)
    .replace(/[-_]+/gu, " ")
    .replace(/\b(\d+)b\b/giu, "$1B")
    .replace(/\bai\b/giu, "AI")
    .replace(/\bllama\b/giu, "Llama")
    .replace(/\bqwen\b/giu, "Qwen")
    .replace(/\bgpt\b/giu, "GPT")
    .replace(/\b\w/gu, (letter) => letter.toUpperCase());
  const organisationLabel = organisationNames[organisation.toLowerCase()] ?? organisation;
  return modelParts.length ? `${model} — ${organisationLabel}` : model;
}
