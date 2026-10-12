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
const GROQ_GPT_OSS_NOTE: &str = "Modelos gpt-oss na Groq aceitam esforço baixo, médio e alto (guias de terceiros; confirme no seu painel da Groq). Se o servidor recusar o parâmetro, o app repete a chamada sem ele.";
const GROQ_QWEN_NOTE: &str = "Qwen3 na Groq aceita “sem raciocínio” (none) ou “padrão” (default). Se o servidor recusar o parâmetro, o app repete a chamada sem ele.";
const DEEPSEEK_NOTE: &str = "DeepSeek: modo de pensamento ligado/desligado e esforço alto/máximo (guia oficial de Thinking Mode). Sem raciocínio é o padrão para revisão de página. Se o modelo recusar o parâmetro, o app repete a chamada sem ele.";
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

fn options(levels: &[(&str, &str)], default: &str, note: &str) -> ModelEffortOptions {
    ModelEffortOptions { configurable: true, levels: levels.iter().map(|(value, label)| EffortOption { value: value.to_string(), label: label.to_string() }).collect(), default: default.to_string(), note: note.to_string() }
}

/// Devolve as opções de esforço aplicáveis a `provider`/`model`.
pub fn effort_options(provider: &str, model: &str) -> ModelEffortOptions {
    let model_lower = model.trim().to_ascii_lowercase();
    if provider == "groq" && model_lower.contains("gpt-oss") { return options(&[("low", "Baixo"), ("medium", "Médio"), ("high", "Alto")], "low", GROQ_GPT_OSS_NOTE); }
    if provider == "groq" && model_lower.contains("qwen3") { return options(&[("none", "Sem raciocínio"), ("default", "Padrão")], "none", GROQ_QWEN_NOTE); }
    if provider == "deepseek" { return options(&[("off", "Sem raciocínio"), ("high", "Alto"), ("max", "Máximo")], "off", DEEPSEEK_NOTE); }
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
    if provider == "openai" {
        // Modelos de raciocínio da OpenAI (o1/o3/o4 e gpt-5.x) rejeitam
        // `max_tokens` e `temperature` diferente do padrão; exigem
        // `max_completion_tokens`. O orçamento inclui o raciocínio.
        let id = model.trim().to_ascii_lowercase();
        let is_reasoning = id.starts_with("o1") || id.starts_with("o3") || id.starts_with("o4") || id.starts_with("gpt-5");
        if is_reasoning {
            if let Some(object) = body.as_object_mut() {
                object.remove("temperature");
                let budget = object.remove("max_tokens").and_then(|v| v.as_u64()).unwrap_or(1400);
                object.insert("max_completion_tokens".to_string(), json!(budget.max(4096)));
            }
        }
        return Ok(());
    }
    if provider == "groq" {
        let id = model.trim().to_ascii_lowercase();
        let (levels, default): (&[&str], &str) = if id.contains("gpt-oss") { (&["low", "medium", "high"], "low") } else if id.contains("qwen3") { (&["none", "default"], "none") } else { return Ok(()); };
        let level = reasoning_effort.unwrap_or(default);
        if !levels.contains(&level) { return Err(format!("O nível de esforço “{level}” não é suportado por {model} na Groq. Nenhuma conversão foi aplicada.")); }
        body["reasoning_effort"] = json!(level);
        if level != "none" { body["max_tokens"] = json!(if level == "high" { 8192 } else { 4096 }); }
        return Ok(());
    }
    if provider == "deepseek" {
        match reasoning_effort.unwrap_or("off") {
            "off" => { body["thinking"] = json!({ "type": "disabled" }); }
            level @ ("high" | "max") => {
                body["thinking"] = json!({ "type": "enabled" });
                body["reasoning_effort"] = json!(level);
                body["max_tokens"] = json!(if level == "max" { 16384 } else { 8192 });
            }
            other => return Err(format!("O nível de esforço “{other}” não é suportado pela DeepSeek. Níveis: sem raciocínio, alto e máximo. Nenhuma conversão foi aplicada.")),
        }
        return Ok(());
    }
    if provider != "nvidia" {
        // Demais provedores seguem o caminho comum até que uma família seja
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

/// Remove os parâmetros de raciocínio (usado para repetir a chamada quando o servidor os recusa).
pub fn strip_effort_parameters(body: &mut Value) -> bool {
    let Some(object) = body.as_object_mut() else { return false };
    let removed = object.remove("reasoning_effort").is_some() | object.remove("thinking").is_some();
    removed
}

/// O servidor respondeu 400/422 reclamando de um parâmetro de raciocínio?
pub fn effort_rejected(status: u16, payload: &Value) -> bool {
    if status != 400 && status != 422 { return false; }
    let text = payload.pointer("/error/message").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| payload.to_string()).to_ascii_lowercase();
    text.contains("reasoning_effort") || text.contains("thinking") || text.contains("reasoning")
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
    fn openai_reasoning_models_use_completion_token_budget() {
        let mut body = json!({ "max_tokens": 1400, "temperature": 0.2 });
        apply_model_parameters(&mut body, "openai", "gpt-5-mini", None).unwrap();
        assert!(body.get("max_tokens").is_none() && body.get("temperature").is_none());
        assert_eq!(body["max_completion_tokens"], 4096);
        let mut classic = json!({ "max_tokens": 1400, "temperature": 0.2 });
        apply_model_parameters(&mut classic, "openai", "gpt-4o", None).unwrap();
        assert_eq!(classic["max_tokens"], 1400);
        assert_eq!(classic["temperature"], 0.2);
    }

    #[test]
    fn groq_gpt_oss_and_qwen3_expose_their_levels() {
        let oss = effort_options("groq", "openai/gpt-oss-120b");
        assert_eq!(oss.levels.iter().map(|l| l.value.as_str()).collect::<Vec<_>>(), vec!["low", "medium", "high"]);
        assert_eq!(oss.default, "low");
        let qwen = effort_options("groq", "qwen/qwen3-32b");
        assert_eq!(qwen.levels.iter().map(|l| l.value.as_str()).collect::<Vec<_>>(), vec!["none", "default"]);
        assert!(!effort_options("groq", "llama-3.3-70b-versatile").configurable);
        let mut body = json!({ "max_tokens": 1400 });
        apply_model_parameters(&mut body, "groq", "openai/gpt-oss-20b", Some("high")).unwrap();
        assert_eq!((body["reasoning_effort"].as_str(), body["max_tokens"].as_u64()), (Some("high"), Some(8192)));
        let mut qwen_body = json!({ "max_tokens": 1400 });
        apply_model_parameters(&mut qwen_body, "groq", "qwen/qwen3-32b", None).unwrap();
        assert_eq!((qwen_body["reasoning_effort"].as_str(), qwen_body["max_tokens"].as_u64()), (Some("none"), Some(1400)));
        let mut llama = json!({ "max_tokens": 1400 });
        apply_model_parameters(&mut llama, "groq", "llama-3.3-70b-versatile", Some("low")).unwrap();
        assert!(llama.get("reasoning_effort").is_none());
        let mut bad = json!({});
        assert!(apply_model_parameters(&mut bad, "groq", "openai/gpt-oss-20b", Some("none")).is_err(), "gpt-oss não tem 'none'");
    }

    #[test]
    fn deepseek_defaults_to_thinking_off_and_maps_levels() {
        let options = effort_options("deepseek", "deepseek-chat");
        assert_eq!((options.configurable, options.default.as_str()), (true, "off"));
        let mut body = json!({ "max_tokens": 1400 });
        apply_model_parameters(&mut body, "deepseek", "deepseek-chat", None).unwrap();
        assert_eq!(body["thinking"]["type"], "disabled");
        assert!(body.get("reasoning_effort").is_none());
        let mut high = json!({ "max_tokens": 1400 });
        apply_model_parameters(&mut high, "deepseek", "deepseek-reasoner", Some("max")).unwrap();
        assert_eq!((high["thinking"]["type"].as_str(), high["reasoning_effort"].as_str(), high["max_tokens"].as_u64()), (Some("enabled"), Some("max"), Some(16384)));
        assert!(apply_model_parameters(&mut json!({}), "deepseek", "deepseek-chat", Some("medium")).is_err());
    }

    #[test]
    fn rejection_detection_and_stripping_allow_a_safe_retry() {
        let rejected = json!({ "error": { "message": "Unknown parameter: reasoning_effort for this model" } });
        assert!(effort_rejected(400, &rejected) && effort_rejected(422, &rejected));
        assert!(!effort_rejected(401, &rejected) && !effort_rejected(500, &rejected));
        assert!(!effort_rejected(400, &json!({ "error": { "message": "invalid api key" } })));
        let mut body = json!({ "model": "x", "reasoning_effort": "low", "thinking": { "type": "enabled" }, "max_tokens": 10 });
        assert!(strip_effort_parameters(&mut body));
        assert_eq!(body, json!({ "model": "x", "max_tokens": 10 }));
        assert!(!strip_effort_parameters(&mut body));
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
