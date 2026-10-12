export type AiProviderKind = "nvidia-nim" | "groq" | "openai" | "openai-compatible";
export interface AiProviderConfig { id: string; kind: AiProviderKind; label: string; baseUrl: string; model: string; secretRef: string; }
export const nvidiaNimDefaults = { kind: "nvidia-nim" as const, label: "NVIDIA NIM", baseUrl: "https://integrate.api.nvidia.com/v1", chatPath: "/chat/completions" };
export interface ChatMessage { role: "system" | "user" | "assistant"; content: string; }
export interface LanguageModelGateway { complete(input: { provider: AiProviderConfig; messages: ChatMessage[]; signal?: AbortSignal }): Promise<string>; }
