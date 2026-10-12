//! Capacidades e parâmetros de raciocínio por família de modelo.
//!
//! Este módulo é a fonte única de verdade sobre quais níveis de esforço um
//! modelo expõe e como traduzi-los em parâmetros de requisição. Ele existe para
//! que `lib.rs` não acumule conhecimento específico de cada provedor/modelo.
//!
//! Regras seguidas aqui:
//! - Só enviamos um parâmetro quando a documentação oficial da família o
//!   descreve. Não presumimos nomes, níveis ou valores iguais entre famílias.
//! - Se um nível não for suportado, retornamos erro explícito em vez de
//!   converter silenciosamente para outro nível.
//! - O padrão para revisão de página é sempre o esforço mais baixo disponível.

use serde::Serialize;
use serde_json::{json, Value};

/// Uma opção de esforço oferecida na interface. `value` é o valor exato que a
/// API documenta (não traduzido); `label` é a apresentação em pt-BR.
#[derive(Debug, Clone, Serialize)]
pub struct EffortOption {
    pub value: String,
    pub label: String,
}

/// Resposta do comando `get_model_effort_options`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEffortOptions {
    /// `true` quando o modelo documenta níveis de esforço controláveis.
    pub configurable: bool,
    /// Níveis exibidos, na ordem documentada. Vazio quando não configurável.
    pub levels: Vec<EffortOption>,
    /// Valor padrão (usado para revisão de página e quando nada foi escolhido).
    pub default: String,
    /// Explicação curta em pt-BR; vazia quando não há ressalva a exibir.
    pub note: String,
}

const GLM_NOTE: &str =
    "Níveis documentados pela Z.AI: baixo, alto e máximo. “Médio” não é suportado por este modelo.";
const NEMOTRON_NOTE: &str = "Este modelo controla o modo de pensamento (enable_thinking), não níveis de esforço. Para revisão de página, o app usa o modo sem raciocínio.";
const DEFAULT_NOTE: &str = "Este modelo não documenta níveis de esforço de raciocínio; os parâmetros padrão do provedor são usados.";

fn glm_effort_options() -> ModelEffortOptions {
    ModelEffortOptions {
        configurable: true,
        levels: vec![
            EffortOption { value: "low".to_string(), label: "Baixo".to_string() },
            EffortOption { value: "high".to_string(), label: "Alto".to_string() },
            EffortOption { value: "max".to_string(), label: "Máximo".to_string() },
        ],
        default: "low".to_string(),
        note: GLM_NOTE.to_string(),
    }
}

fn automatic_effort_options(note: &'static str) -> ModelEffortOptions {
    ModelEffortOptions {
        configurable: false,
        levels: Vec::new(),
        default: "auto".to_string(),
        note: note.to_string(),
    }
}

/// Devolve as opções de esforço aplicáveis a `provider`/`model`.
pub fn effort_options(provider: &str, model: &str) -> ModelEffortOptions {
    let model_lower = model.trim().to_ascii_lowercase();
    if provider == "nvidia" && model_lower.starts_with("z-ai/glm") {
        return glm_effort_options();
    }
    if provider == "nvidia"
        && (model_lower.starts_with("nvidia/nemotron-3-super")
            || model_lower.starts_with("nvidia/nemotron-3-ultra"))
    {
        return automatic_effort_options(NEMOTRON_NOTE);
    }
    automatic_effort_options(DEFAULT_NOTE)
}

/// Aplica os parâmetros específicos do modelo ao corpo da requisição.
///
/// `reasoning_effort` é o valor escolhido na interface (já validado contra os
/// níveis documentados) ou `None` para usar o padrão de revisão de página.
pub fn apply_model_parameters(
    body: &mut Value,
    provider: &str,
    model: &str,
    reasoning_effort: Option<&str>,
) -> Result<(), String> {
    if provider != "nvidia" {
        // Groq e OpenAI seguem o caminho comum até que uma família seja
        // verificada ao vivo. Não enviamos reasoning_effort para eles.
        return Ok(());
    }
    let model_lower = model.trim().to_ascii_lowercase();

    if model_lower.starts_with("z-ai/glm") {
        // GLM-5.x (ex.: z-ai/glm-5.3-flash) documenta reasoning_effort com os
        // níveis `low`, `high` e `max` (padrão `max`). O chat template usa
        // `clear_thinking` e Z.AI pede `clear_thinking=true` para cenários de
        // chat. O orçamento de conclusão inclui o raciocínio; `low` foi
        // verificado ao vivo com 4096 tokens, e níveis maiores recebem mais
        // margem para a cadeia de raciocínio.
        let level = match reasoning_effort {
            None => "low",
            Some("low") => "low",
            Some("high") => "high",
            Some("max") => "max",
            Some(other) => {
                return Err(format!(
                    "O nível de esforço “{other}” não é suportado por {model}. Níveis documentados: baixo, alto e máximo. Nenhuma conversão foi aplicada."
                ));
            }
        };
        body["reasoning_effort"] = json!(level);
        body["chat_template_kwargs"] = json!({ "clear_thinking": true });
        body["max_tokens"] = json!(if level == "low" { 4096 } else { 8192 });
        return Ok(());
    }

    if model_lower.starts_with("nvidia/nemotron-3-super")
        || model_lower.starts_with("nvidia/nemotron-3-ultra")
    {
        // Nemotron 3 Super/Ultra expõem um interruptor de pensamento, não
        // níveis de esforço. A revisão de página roda com pensamento
        // desativado e a amostragem recomendada para esses modelos.
        body["chat_template_kwargs"] = json!({ "enable_thinking": false });
        body["temperature"] = json!(1.0);
        body["top_p"] = json!(0.95);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glm_exposes_only_documented_levels() {
        let options = effort_options("nvidia", "z-ai/glm-5.3-flash");
        assert!(options.configurable);
        assert_eq!(options.default, "low");
        let values: Vec<&str> = options.levels.iter().map(|level| level.value.as_str()).collect();
        assert_eq!(values, vec!["low", "high", "max"]);
        assert!(!values.contains(&"medium"), "GLM não documenta o nível médio");
    }

    #[test]
    fn nemotron_and_unknown_models_are_not_configurable() {
        assert!(!effort_options("nvidia", "nvidia/nemotron-3-super-120b-a12b").configurable);
        assert!(!effort_options("nvidia", "nvidia/nemotron-3-ultra-550b-a55b").configurable);
        let unknown = effort_options("nvidia", "meta/llama-3.1-8b-instruct");
        assert!(!unknown.configurable);
        assert_eq!(unknown.default, "auto");
    }

    #[test]
    fn glm_low_sets_verified_parameters() {
        let mut body = json!({ "max_tokens": 1400 });
        apply_model_parameters(&mut body, "nvidia", "z-ai/glm-5.3-flash", Some("low")).unwrap();
        assert_eq!(body["reasoning_effort"], "low");
        assert_eq!(body["chat_template_kwargs"]["clear_thinking"], true);
        assert_eq!(body["max_tokens"], 4096);
    }

    #[test]
    fn glm_high_and_max_get_larger_budget() {
        for level in ["high", "max"] {
            let mut body = json!({ "max_tokens": 1400 });
            apply_model_parameters(&mut body, "nvidia", "z-ai/glm-5.3-flash", Some(level)).unwrap();
            assert_eq!(body["reasoning_effort"], level);
            assert_eq!(body["chat_template_kwargs"]["clear_thinking"], true);
            assert_eq!(body["max_tokens"], 8192);
        }
    }

    #[test]
    fn glm_defaults_to_low_for_page_review() {
        let mut body = json!({ "max_tokens": 1400 });
        apply_model_parameters(&mut body, "nvidia", "z-ai/glm-5.3-flash", None).unwrap();
        assert_eq!(body["reasoning_effort"], "low");
    }

    #[test]
    fn glm_rejects_unsupported_level_instead_of_converting() {
        let mut body = json!({ "max_tokens": 1400 });
        let error = apply_model_parameters(&mut body, "nvidia", "z-ai/glm-5.3-flash", Some("medium"))
            .expect_err("nível médio não deve ser aceito silenciosamente");
        assert!(error.contains("não é suportado"));
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn nemotron_disables_thinking_and_sets_recommended_sampling() {
        let mut body = json!({ "max_tokens": 1400 });
        apply_model_parameters(&mut body, "nvidia", "nvidia/nemotron-3-super-120b-a12b", None).unwrap();
        assert_eq!(body["chat_template_kwargs"]["enable_thinking"], false);
        assert_eq!(body["temperature"], 1.0);
        assert_eq!(body["top_p"], 0.95);
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn other_providers_do_not_receive_effort_parameters() {
        let mut body = json!({ "max_tokens": 1400 });
        apply_model_parameters(&mut body, "groq", "z-ai/glm-5.3-flash", Some("low")).unwrap();
        apply_model_parameters(&mut body, "openai", "gpt-4o", Some("low")).unwrap();
        assert!(body.get("reasoning_effort").is_none());
        assert!(body.get("chat_template_kwargs").is_none());
    }
}
