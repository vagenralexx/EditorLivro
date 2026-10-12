//! Registro dos provedores de IA suportados (todos com API compatível com o
//! formato OpenAI `chat/completions`) e leitura do catálogo de modelos.
//!
//! Para adicionar um provedor: inclua uma entrada em `PROVIDERS` aqui e o
//! mesmo `id` em `apps/desktop/src/features/ai/providers.ts`.

use serde_json::Value;

pub struct ProviderSpec {
    pub id: &'static str,
    pub chat_endpoint: &'static str,
    pub models_endpoint: &'static str,
    /// Alguns provedores listam modelos sem autenticação (ex.: OpenRouter). Nesses
    /// casos a chave é validada por este endpoint, que responde 401 se for inválida.
    pub key_check_endpoint: Option<&'static str>,
}

pub const PROVIDERS: &[ProviderSpec] = &[
    ProviderSpec { id: "nvidia", chat_endpoint: "https://integrate.api.nvidia.com/v1/chat/completions", models_endpoint: "https://integrate.api.nvidia.com/v1/models", key_check_endpoint: None },
    ProviderSpec { id: "groq", chat_endpoint: "https://api.groq.com/openai/v1/chat/completions", models_endpoint: "https://api.groq.com/openai/v1/models", key_check_endpoint: None },
    ProviderSpec { id: "openai", chat_endpoint: "https://api.openai.com/v1/chat/completions", models_endpoint: "https://api.openai.com/v1/models", key_check_endpoint: None },
    ProviderSpec { id: "openrouter", chat_endpoint: "https://openrouter.ai/api/v1/chat/completions", models_endpoint: "https://openrouter.ai/api/v1/models", key_check_endpoint: Some("https://openrouter.ai/api/v1/auth/key") },
    ProviderSpec { id: "gemini", chat_endpoint: "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions", models_endpoint: "https://generativelanguage.googleapis.com/v1beta/openai/models", key_check_endpoint: None },
    ProviderSpec { id: "mistral", chat_endpoint: "https://api.mistral.ai/v1/chat/completions", models_endpoint: "https://api.mistral.ai/v1/models", key_check_endpoint: None },
    ProviderSpec { id: "deepseek", chat_endpoint: "https://api.deepseek.com/chat/completions", models_endpoint: "https://api.deepseek.com/models", key_check_endpoint: None },
    ProviderSpec { id: "xai", chat_endpoint: "https://api.x.ai/v1/chat/completions", models_endpoint: "https://api.x.ai/v1/models", key_check_endpoint: None },
    ProviderSpec { id: "together", chat_endpoint: "https://api.together.xyz/v1/chat/completions", models_endpoint: "https://api.together.xyz/v1/models", key_check_endpoint: None },
    ProviderSpec { id: "cerebras", chat_endpoint: "https://api.cerebras.ai/v1/chat/completions", models_endpoint: "https://api.cerebras.ai/v1/models", key_check_endpoint: None },
];

pub fn spec(provider: &str) -> Result<&'static ProviderSpec, String> {
    PROVIDERS.iter().find(|spec| spec.id == provider).ok_or_else(|| "Provedor não suportado.".to_string())
}

/// O catálogo dos provedores mistura modelos de chat com embeddings, áudio,
/// moderação e imagem. Esses não servem para análise editorial e poluem o seletor.
pub fn is_chat_model(id: &str) -> bool {
    const EXCLUDED: &[&str] = &["embed", "rerank", "whisper", "tts", "transcribe", "speech", "moderation", "guard", "dall-e", "image", "audio", "realtime", "clip", "ocr", "parse", "safety", "reward", "imagen", "veo", "sora"];
    let id = id.to_ascii_lowercase();
    !EXCLUDED.iter().any(|marker| id.contains(marker))
}

/// Extrai os ids de modelos de uma resposta de catálogo. Aceita o formato
/// OpenAI (`{"data":[{"id":..}]}`), listas puras (Together) e ids com prefixo
/// `models/` (Gemini), que o endpoint de chat não aceita.
pub fn parse_models(payload: &Value) -> Vec<String> {
    let items = payload.get("data").and_then(Value::as_array).or_else(|| payload.as_array());
    let mut ids: Vec<String> = items.into_iter().flatten().filter_map(|item| {
        // Together informa o tipo do modelo; só interessam os de chat.
        if let Some(kind) = item.get("type").and_then(Value::as_str) {
            if !matches!(kind, "chat" | "language") { return None; }
        }
        item.get("id").and_then(Value::as_str).map(|id| id.strip_prefix("models/").unwrap_or(id).to_string())
    }).collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// Aplica o filtro de chat, com fallback para o catálogo completo se ele esvaziar a lista.
pub fn chat_models(all: Vec<String>) -> Vec<String> {
    let filtered: Vec<String> = all.iter().filter(|id| is_chat_model(id)).cloned().collect();
    if filtered.is_empty() { all } else { filtered }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_provider_is_known_and_has_https_endpoints() {
        let ids: Vec<&str> = PROVIDERS.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec!["nvidia", "groq", "openai", "openrouter", "gemini", "mistral", "deepseek", "xai", "together", "cerebras"]);
        for provider in PROVIDERS {
            assert!(provider.chat_endpoint.starts_with("https://") && provider.models_endpoint.starts_with("https://"), "{}", provider.id);
            assert!(spec(provider.id).is_ok());
        }
        assert!(spec("desconhecido").is_err());
    }

    #[test]
    fn chat_model_filter_drops_non_chat_models() {
        assert!(is_chat_model("meta/llama-3.3-70b-instruct"));
        assert!(is_chat_model("gpt-4o"));
        assert!(!is_chat_model("nvidia/nv-embedqa-e5-v5"));
        assert!(!is_chat_model("whisper-large-v3"));
        assert!(!is_chat_model("meta-llama/llama-guard-4-12b"));
        assert_eq!(chat_models(vec!["text-embedding-3".into()]), vec!["text-embedding-3".to_string()], "sem modelos de chat, mostra o catálogo completo");
    }

    #[test]
    fn parses_openai_style_list() {
        let payload = json!({"object":"list","data":[{"id":"b"},{"id":"a"},{"id":"a"},{"nome":"x"}]});
        assert_eq!(parse_models(&payload), vec!["a", "b"]);
    }

    #[test]
    fn parses_together_top_level_array_and_keeps_only_chat() {
        let payload = json!([{"id":"meta/llama","type":"chat"},{"id":"emb","type":"embedding"},{"id":"img","type":"image"}]);
        assert_eq!(parse_models(&payload), vec!["meta/llama"]);
    }

    #[test]
    fn strips_gemini_models_prefix() {
        let payload = json!({"data":[{"id":"models/gemini-2.0-flash"},{"id":"models/text-embedding-004"}]});
        let ids = chat_models(parse_models(&payload));
        assert_eq!(ids, vec!["gemini-2.0-flash"]);
    }
}
