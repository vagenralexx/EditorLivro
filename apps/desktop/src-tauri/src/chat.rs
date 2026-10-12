//! Chamadas de chat com resposta em streaming (SSE).
//!
//! Por que streaming: um teto fixo para a resposta inteira derruba modelos que
//! funcionam, mas demoram (fila em planos gratuitos, modelos que "pensam" antes
//! de responder). Aqui o limite é de INATIVIDADE: enquanto o servidor envia
//! algo (inclusive o raciocínio), a análise continua. O usuário vê o progresso
//! e pode cancelar.

use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::Serialize;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct ChatLimits {
    /// Espera máxima até o servidor começar a responder (fila).
    pub first_byte: Duration,
    /// Silêncio máximo entre dois pedaços da resposta.
    pub idle: Duration,
    /// Teto absoluto, para nunca ficar preso indefinidamente.
    pub total: Duration,
}

pub const DEFAULT_LIMITS: ChatLimits = ChatLimits {
    first_byte: Duration::from_secs(300),
    idle: Duration::from_secs(120),
    total: Duration::from_secs(900),
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub elapsed_ms: u64,
    pub answer_chars: usize,
    pub reasoning_chars: usize,
    /// "waiting" (fila), "thinking" (raciocínio) ou "writing" (resposta).
    pub phase: &'static str,
}

#[derive(Debug)]
pub enum ChatError {
    Network(String),
    Http { status: u16, detail: String, body: Value },
    Provider(String),
    Cancelled,
    FirstByteTimeout(Duration),
    IdleTimeout(Duration),
    TotalTimeout(Duration),
}

impl ChatError {
    pub fn message(&self) -> String {
        match self {
            ChatError::Network(text) | ChatError::Provider(text) => text.clone(),
            ChatError::Http { status, detail, .. } => format!("O provedor respondeu {status}: {detail}"),
            ChatError::Cancelled => "Análise cancelada.".to_string(),
            ChatError::FirstByteTimeout(limit) => format!("O servidor não começou a responder em {} s (fila cheia ou modelo indisponível). Tente outro modelo ou servidor.", limit.as_secs()),
            ChatError::IdleTimeout(limit) => format!("O servidor parou de enviar dados por {} s. A análise foi interrompida; tente novamente ou escolha outro modelo.", limit.as_secs()),
            ChatError::TotalTimeout(limit) => format!("A análise passou de {} min sem terminar. Escolha um modelo mais rápido ou reduza o nível de raciocínio.", limit.as_secs() / 60),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ChatOutput {
    pub text: String,
    pub finish_reason: Option<String>,
    pub reasoning_chars: usize,
}

/// Interpreta o fluxo SSE aos pedaços (linhas podem chegar partidas).
#[derive(Default)]
pub struct SseAccumulator {
    buffer: String,
    pub output: ChatOutput,
    pub error: Option<String>,
    pub done: bool,
}

impl SseAccumulator {
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buffer.push_str(&String::from_utf8_lossy(chunk));
        while let Some(newline) = self.buffer.find('\n') {
            let line: String = self.buffer.drain(..=newline).collect();
            self.handle_line(line.trim_end_matches(['\r', '\n']));
        }
    }

    pub fn finish(&mut self) {
        let rest = std::mem::take(&mut self.buffer);
        if !rest.trim().is_empty() { self.handle_line(rest.trim_end()); }
    }

    fn handle_line(&mut self, line: &str) {
        let Some(data) = line.strip_prefix("data:") else { return };
        let data = data.trim();
        if data == "[DONE]" { self.done = true; return; }
        let Ok(value) = serde_json::from_str::<Value>(data) else { return };
        if let Some(message) = value.pointer("/error/message").and_then(Value::as_str).or_else(|| value.get("error").and_then(Value::as_str)) {
            self.error = Some(message.to_string());
            return;
        }
        let choice = value.pointer("/choices/0");
        if let Some(text) = choice.and_then(|c| c.pointer("/delta/content")).and_then(Value::as_str) {
            self.output.text.push_str(text);
        }
        for key in ["reasoning_content", "reasoning"] {
            if let Some(text) = choice.and_then(|c| c.pointer(&format!("/delta/{key}"))).and_then(Value::as_str) {
                self.output.reasoning_chars += text.chars().count();
            }
        }
        if let Some(reason) = choice.and_then(|c| c.get("finish_reason")).and_then(Value::as_str) {
            self.output.finish_reason = Some(reason.to_string());
        }
    }
}

/// Remove blocos `<think>…</think>` que alguns modelos incluem no texto final.
pub fn strip_think_blocks(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        match rest[start..].find("</think>") {
            Some(end) => rest = &rest[start + end + "</think>".len()..],
            None => { rest = ""; break; }
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}

fn phase(output: &ChatOutput, started_receiving: bool) -> &'static str {
    if !output.text.is_empty() { "writing" } else if output.reasoning_chars > 0 || started_receiving { "thinking" } else { "waiting" }
}

/// Envia a requisição e lê a resposta em streaming. `on_progress` é chamado
/// cerca de uma vez por segundo e a cada mudança de fase.
pub async fn stream_chat(
    client: &reqwest::Client,
    endpoint: &str,
    api_key: &str,
    body: &Value,
    limits: ChatLimits,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(Progress),
) -> Result<ChatOutput, ChatError> {
    let started = Instant::now();
    let tick = Duration::from_millis(500);
    let request = client.post(endpoint)
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .header(ACCEPT, "text/event-stream, application/json")
        .header(CONTENT_TYPE, "application/json")
        .json(body)
        .send();
    tokio::pin!(request);
    // Fase 1: aguardando os cabeçalhos (fila), com cancelamento e progresso.
    let response = loop {
        tokio::select! {
            result = &mut request => break result.map_err(|error| ChatError::Network(crate::describe_network_error(&error)))?,
            _ = tokio::time::sleep(tick) => {
                if cancel.load(Ordering::Relaxed) { return Err(ChatError::Cancelled); }
                if started.elapsed() >= limits.first_byte { return Err(ChatError::FirstByteTimeout(limits.first_byte)); }
                on_progress(Progress { elapsed_ms: started.elapsed().as_millis() as u64, answer_chars: 0, reasoning_chars: 0, phase: "waiting" });
            }
        }
    };
    let status = response.status();
    let is_stream = response.headers().get(CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(|v| v.contains("text/event-stream")).unwrap_or(false);

    let mut accumulator = SseAccumulator::default();
    let mut raw = Vec::new();
    let mut last_data = Instant::now();
    let mut last_progress = Instant::now();
    let mut response = response;
    loop {
        if cancel.load(Ordering::Relaxed) { return Err(ChatError::Cancelled); }
        if started.elapsed() >= limits.total { return Err(ChatError::TotalTimeout(limits.total)); }
        match tokio::time::timeout(tick, response.chunk()).await {
            Err(_) => {
                let waiting_first = accumulator.output.text.is_empty() && accumulator.output.reasoning_chars == 0 && raw.is_empty();
                if waiting_first && started.elapsed() >= limits.first_byte { return Err(ChatError::FirstByteTimeout(limits.first_byte)); }
                if !waiting_first && last_data.elapsed() >= limits.idle { return Err(ChatError::IdleTimeout(limits.idle)); }
            }
            Ok(Err(error)) => return Err(ChatError::Network(crate::describe_network_error(&error))),
            Ok(Ok(None)) => break,
            Ok(Ok(Some(bytes))) => {
                last_data = Instant::now();
                if status.is_success() && is_stream { accumulator.feed(&bytes); } else { raw.extend_from_slice(&bytes); }
                if let Some(message) = accumulator.error.take() { return Err(ChatError::Provider(format!("O provedor interrompeu a resposta: {message}"))); }
                if accumulator.done { break; }
            }
        }
        if last_progress.elapsed() >= Duration::from_secs(1) {
            last_progress = Instant::now();
            on_progress(Progress { elapsed_ms: started.elapsed().as_millis() as u64, answer_chars: accumulator.output.text.chars().count(), reasoning_chars: accumulator.output.reasoning_chars, phase: phase(&accumulator.output, true) });
        }
    }
    accumulator.finish();

    if status.is_success() && is_stream {
        return Ok(accumulator.output);
    }
    let payload: Value = serde_json::from_slice(&raw).unwrap_or(Value::Null);
    if !status.is_success() {
        let detail = payload.pointer("/error/message").and_then(Value::as_str).or_else(|| payload.get("message").and_then(Value::as_str)).unwrap_or("sem detalhes retornados").to_string();
        return Err(ChatError::Http { status: status.as_u16(), detail, body: payload });
    }
    // Sucesso sem streaming (alguns provedores ignoram `stream`): resposta JSON completa.
    let choice = payload.pointer("/choices/0").cloned().unwrap_or(Value::Null);
    let reasoning = ["reasoning_content", "reasoning"].iter().filter_map(|key| choice.pointer(&format!("/message/{key}")).and_then(Value::as_str)).map(|t| t.chars().count()).max().unwrap_or(0);
    Ok(ChatOutput {
        text: crate::response_text(&choice["message"]["content"]).or_else(|| crate::response_text(&choice["text"])).unwrap_or_default(),
        finish_reason: choice.get("finish_reason").and_then(Value::as_str).map(str::to_string),
        reasoning_chars: reasoning,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn accumulator_handles_split_lines_reasoning_and_done() {
        let mut acc = SseAccumulator::default();
        acc.feed(b"data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"pensan");
        acc.feed(b"do\"}}]}\r\n\r\ndata: {\"choices\":[{\"delta\":{\"content\":\"Ol\"}}]}\n");
        acc.feed(b"data: {\"choices\":[{\"delta\":{\"content\":\"\\u00e1\"},\"finish_reason\":\"stop\"}]}\ndata: [DONE]\n");
        assert_eq!(acc.output.text, "Olá");
        assert_eq!(acc.output.reasoning_chars, 8);
        assert_eq!(acc.output.finish_reason.as_deref(), Some("stop"));
        assert!(acc.done && acc.error.is_none());
    }

    #[test]
    fn accumulator_reports_in_stream_errors_and_ignores_noise() {
        let mut acc = SseAccumulator::default();
        acc.feed(b": keep-alive\n\nevent: ping\ndata: not-json\ndata: {\"error\":{\"message\":\"sobrecarga\"}}\n");
        assert_eq!(acc.error.as_deref(), Some("sobrecarga"));
    }

    #[test]
    fn think_blocks_are_removed() {
        assert_eq!(strip_think_blocks("<think>{raciocínio}</think>\n{\"a\":1}"), "{\"a\":1}");
        assert_eq!(strip_think_blocks("sem bloco"), "sem bloco");
        assert_eq!(strip_think_blocks("antes <think>nunca fecha"), "antes");
    }

    /// Servidor HTTP mínimo: para cada conexão executa `script` (que escreve a resposta).
    fn serve(script: impl Fn(&mut std::net::TcpStream) + Send + 'static) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0u8; 8192];
                let _ = stream.read(&mut buffer);
                script(&mut stream);
            }
        });
        format!("http://{address}/v1/chat/completions")
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()
    }

    fn sse_head(stream: &mut std::net::TcpStream) {
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").unwrap();
    }

    fn write_chunk(stream: &mut std::net::TcpStream, text: &str) {
        write!(stream, "{:x}\r\n{}\r\n", text.len(), text).unwrap();
        stream.flush().unwrap();
    }

    fn limits(first: u64, idle: u64) -> ChatLimits {
        ChatLimits { first_byte: Duration::from_secs(first), idle: Duration::from_secs(idle), total: Duration::from_secs(30) }
    }

    #[test]
    fn slow_but_progressing_stream_is_not_cut_by_idle_limit() {
        // 4 pedaços, 1 s entre eles: total ~4 s, maior que o limite de inatividade de 2 s.
        let url = serve(|stream| {
            sse_head(stream);
            for part in ["{\\\"a\\\":", "1,", "\\\"b\\\":", "2}"] {
                std::thread::sleep(Duration::from_millis(1000));
                write_chunk(stream, &format!("data: {{\"choices\":[{{\"delta\":{{\"content\":\"{part}\"}}}}]}}\n\n"));
            }
            write_chunk(stream, "data: [DONE]\n\n");
            stream.write_all(b"0\r\n\r\n").unwrap();
        });
        let cancel = AtomicBool::new(false);
        let mut phases = Vec::new();
        let output = runtime().block_on(stream_chat(&reqwest::Client::new(), &url, "k", &json!({}), limits(10, 2), &cancel, |p| phases.push(p.phase))).unwrap();
        assert_eq!(output.text, "{\"a\":1,\"b\":2}");
        assert!(phases.contains(&"waiting") || phases.contains(&"writing") || phases.contains(&"thinking"));
    }

    #[test]
    fn silent_server_after_first_data_triggers_idle_timeout() {
        let url = serve(|stream| {
            sse_head(stream);
            write_chunk(stream, "data: {\"choices\":[{\"delta\":{\"content\":\"x\"}}]}\n\n");
            std::thread::sleep(Duration::from_secs(6));
        });
        let cancel = AtomicBool::new(false);
        let started = Instant::now();
        let error = runtime().block_on(stream_chat(&reqwest::Client::new(), &url, "k", &json!({}), limits(10, 2), &cancel, |_| {})).unwrap_err();
        assert!(matches!(error, ChatError::IdleTimeout(_)), "{error:?}");
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn queue_wait_beyond_first_byte_limit_is_reported_as_such() {
        let url = serve(|_stream| std::thread::sleep(Duration::from_secs(6)));
        let cancel = AtomicBool::new(false);
        let error = runtime().block_on(stream_chat(&reqwest::Client::new(), &url, "k", &json!({}), limits(1, 2), &cancel, |_| {})).unwrap_err();
        assert!(matches!(error, ChatError::FirstByteTimeout(_)), "{error:?}");
    }

    #[test]
    fn cancel_flag_stops_the_request_quickly() {
        let url = serve(|_stream| std::thread::sleep(Duration::from_secs(6)));
        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        std::thread::spawn(move || { std::thread::sleep(Duration::from_millis(700)); flag.store(true, Ordering::Relaxed); });
        let started = Instant::now();
        let error = runtime().block_on(stream_chat(&reqwest::Client::new(), &url, "k", &json!({}), limits(30, 30), &cancel, |_| {})).unwrap_err();
        assert!(matches!(error, ChatError::Cancelled));
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn plain_json_response_is_supported_for_providers_that_ignore_stream() {
        let url = serve(|stream| {
            let body = "{\"choices\":[{\"message\":{\"content\":\"pronto\",\"reasoning_content\":\"abc\"},\"finish_reason\":\"stop\"}]}";
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let cancel = AtomicBool::new(false);
        let output = runtime().block_on(stream_chat(&reqwest::Client::new(), &url, "k", &json!({}), limits(5, 5), &cancel, |_| {})).unwrap();
        assert_eq!(output.text, "pronto");
        assert_eq!(output.reasoning_chars, 3);
    }

    #[test]
    fn http_errors_carry_status_and_provider_message() {
        let url = serve(|stream| {
            let body = "{\"error\":{\"message\":\"model_not_found\"}}";
            write!(stream, "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        let cancel = AtomicBool::new(false);
        let error = runtime().block_on(stream_chat(&reqwest::Client::new(), &url, "k", &json!({}), limits(5, 5), &cancel, |_| {})).unwrap_err();
        match error { ChatError::Http { status, detail, .. } => { assert_eq!(status, 404); assert_eq!(detail, "model_not_found"); } other => panic!("{other:?}") }
    }
}
