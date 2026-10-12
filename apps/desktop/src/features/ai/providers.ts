export type ProviderId = "nvidia" | "groq" | "openai";

export interface ProviderOption {
  id: ProviderId;
  label: string;
  badge: string;
  endpoint: string;
}

export const providers: ProviderOption[] = [
  {
    id: "nvidia",
    label: "NVIDIA NIM",
    badge: "NIM",
    endpoint: "integrate.api.nvidia.com",
  },
  {
    id: "groq",
    label: "Groq",
    badge: "Groq",
    endpoint: "api.groq.com",
  },
  {
    id: "openai",
    label: "OpenAI",
    badge: "GPT",
    endpoint: "api.openai.com",
  },
];

export function providerFor(id: ProviderId): ProviderOption {
  return providers.find((provider) => provider.id === id) ?? providers[0];
}

const organisationNames: Record<string, string> = {
  "01-ai": "01.AI", "ai21": "AI21", "deepseek-ai": "DeepSeek", "google": "Google", "meta": "Meta", "microsoft": "Microsoft", "mistralai": "Mistral AI", "moonshotai": "Moonshot AI", "nvidia": "NVIDIA", "openai": "OpenAI", "qwen": "Qwen", "z-ai": "Z.AI",
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
