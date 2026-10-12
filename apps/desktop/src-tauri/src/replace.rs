//! Localizar e substituir palavras inteiras no livro (ex.: renomear um personagem).
//!
//! Regras que protegem o texto do autor:
//! - só **palavras inteiras** ("Caio" não casa com "Caiozinho"; as parecidas são contadas e mostradas);
//! - em XHTML só o **texto** é tocado — nunca tags, atributos (`id`, `class`, `href`), comentários,
//!   `<script>`/`<style>` nem entidades;
//! - em Markdown só o texto corrido: alvos de `[[links]]`, código e URLs ficam como estão;
//! - a caixa acompanha o original: "Caio"→"Jordan", "caio"→"jordan", "CAIO"→"JORDAN";
//! - nada é gravado por estas funções (elas só transformam texto e contam).

use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub struct Options { pub ignore_accents: bool, pub preserve_case: bool }

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Sample { pub before: String, pub matched: String, pub after: String, pub replacement: String }

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    /// Ocorrências de palavra inteira (as que serão substituídas).
    pub count: usize,
    /// Palavras que contêm o termo mas não são ele (ficam como estão).
    pub partial: usize,
    /// Formas encontradas (caixa) e quantas vezes: {"Caio": 40, "caio": 2}.
    pub forms: BTreeMap<String, usize>,
    pub partial_words: BTreeMap<String, usize>,
    pub samples: Vec<Sample>,
}

impl Report {
    pub fn merge(&mut self, other: &Report, sample_cap: usize) {
        self.count += other.count;
        self.partial += other.partial;
        for (k, v) in &other.forms { *self.forms.entry(k.clone()).or_default() += v; }
        for (k, v) in &other.partial_words { if self.partial_words.len() < 30 || self.partial_words.contains_key(k) { *self.partial_words.entry(k.clone()).or_default() += v; } }
        for s in &other.samples { if self.samples.len() < sample_cap { self.samples.push(s.clone()); } }
    }
}

const SAMPLE_CONTEXT: usize = 40;
const MAX_SAMPLES_PER_FILE: usize = 4;

fn fold_char(c: char, ignore_accents: bool) -> char {
    let lower = c.to_lowercase().next().unwrap_or(c);
    if !ignore_accents { return lower; }
    match lower {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a', 'é' | 'è' | 'ê' | 'ë' => 'e', 'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o', 'ú' | 'ù' | 'û' | 'ü' => 'u', 'ç' => 'c', 'ñ' => 'n', other => other,
    }
}

fn is_apostrophe(c: char) -> bool { c == '\'' || c == '’' }
fn is_word(c: char) -> bool { c.is_alphanumeric() || c == '_' }

/// O caractere em `i` faz parte de uma palavra? (apóstrofo entre letras também: "Vel'shar").
fn word_char_at(chars: &[char], i: usize) -> bool {
    let c = chars[i];
    if is_word(c) { return true; }
    is_apostrophe(c) && i > 0 && i + 1 < chars.len() && is_word(chars[i - 1]) && is_word(chars[i + 1])
}

/// Ajusta a caixa da substituição à do trecho encontrado.
pub fn adapt_case(matched: &str, replacement: &str, preserve: bool) -> String {
    if !preserve { return replacement.to_string(); }
    let letters: Vec<char> = matched.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() { return replacement.to_string(); }
    if letters.iter().all(|c| c.is_lowercase()) { return replacement.to_lowercase(); }
    if letters.len() > 1 && letters.iter().all(|c| c.is_uppercase()) { return replacement.to_uppercase(); }
    if letters[0].is_uppercase() {
        let mut it = replacement.chars();
        return match it.next() { Some(first) => first.to_uppercase().collect::<String>() + it.as_str(), None => String::new() };
    }
    replacement.to_string()
}

/// Tenta casar o termo em `chars[i..]`. Espaço no termo casa uma ou mais quebras/espaços do texto.
fn match_at(chars: &[char], i: usize, needle: &[char], ignore_accents: bool) -> Option<usize> {
    let (mut p, mut q) = (i, 0);
    while q < needle.len() {
        if needle[q] == ' ' {
            if p >= chars.len() || !chars[p].is_whitespace() { return None; }
            while p < chars.len() && chars[p].is_whitespace() { p += 1; }
            q += 1;
        } else {
            if p >= chars.len() || fold_char(chars[p], ignore_accents) != needle[q] { return None; }
            p += 1; q += 1;
        }
    }
    Some(p)
}

fn needle_chars(find: &str, ignore_accents: bool) -> Vec<char> {
    let mut out: Vec<char> = Vec::new();
    for c in find.trim().chars() {
        if c.is_whitespace() { if out.last() != Some(&' ') { out.push(' '); } } else { out.push(fold_char(c, ignore_accents)); }
    }
    out
}

/// Substitui em um trecho de TEXTO PURO (sem marcação). Atualiza `report`.
fn replace_text(text: &str, find: &str, replace: &str, opts: Options, report: &mut Report, samples_left: &mut usize) -> String {
    let needle = needle_chars(find, opts.ignore_accents);
    if needle.is_empty() || text.is_empty() { return text.to_string(); }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if let Some(end) = match_at(&chars, i, &needle, opts.ignore_accents) {
            let before_ok = i == 0 || !word_char_at(&chars, i - 1);
            let after_ok = end >= chars.len() || !word_char_at(&chars, end);
            // O termo pode começar/terminar em símbolo (ex.: "Vel'shar"): a fronteira só vale entre letras.
            let starts_word = is_word(chars[i]);
            let ends_word = is_word(chars[end - 1]);
            let whole = (!starts_word || before_ok) && (!ends_word || after_ok);
            let matched: String = chars[i..end].iter().collect();
            if whole {
                let replacement = adapt_case(&matched, replace, opts.preserve_case);
                report.count += 1;
                *report.forms.entry(matched.clone()).or_default() += 1;
                if *samples_left > 0 {
                    *samples_left -= 1;
                    let from = i.saturating_sub(SAMPLE_CONTEXT);
                    let to = (end + SAMPLE_CONTEXT).min(chars.len());
                    let squash = |s: String| s.split_whitespace().collect::<Vec<_>>().join(" ");
                    report.samples.push(Sample { before: squash(chars[from..i].iter().collect()), matched: matched.clone(), after: squash(chars[end..to].iter().collect()), replacement: replacement.clone() });
                }
                out.push_str(&replacement);
                i = end;
                continue;
            }
            // Palavra parecida (ex.: "Caiozinho"): conta, mostra e deixa como está.
            let (mut a, mut b) = (i, end);
            while a > 0 && word_char_at(&chars, a - 1) { a -= 1; }
            while b < chars.len() && word_char_at(&chars, b) { b += 1; }
            let word: String = chars[a..b].iter().collect();
            report.partial += 1;
            if report.partial_words.len() < 30 || report.partial_words.contains_key(&word) { *report.partial_words.entry(word).or_default() += 1; }
            out.extend(&chars[i..b]);
            i = b;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn entity_end(s: &str, at: usize) -> Option<usize> {
    let rest = &s[at + 1..];
    let end = rest.char_indices().take(34).find(|(_, c)| !(c.is_ascii_alphanumeric() || *c == '#'))?;
    (end.1 == ';' && end.0 > 0).then_some(at + 1 + end.0 + 1)
}

fn tag_end(src: &str, start: usize) -> usize {
    let bytes = src.as_bytes();
    let mut quote: Option<u8> = None;
    let mut i = start + 1;
    while i < bytes.len() {
        match (quote, bytes[i]) {
            (Some(q), b) if b == q => quote = None,
            (None, b'"') | (None, b'\'') => quote = Some(bytes[i]),
            (None, b'>') => return i + 1,
            _ => {}
        }
        i += 1;
    }
    src.len()
}

/// Texto entre tags: entidades (`&nbsp;`, `&#8217;`) ficam intocadas e separam trechos.
fn replace_xhtml_text(run: &str, find: &str, replace: &str, opts: Options, report: &mut Report, samples_left: &mut usize, out: &mut String) {
    let mut start = 0;
    let mut i = 0;
    while let Some(rel) = run[i..].find('&') {
        let at = i + rel;
        if let Some(end) = entity_end(run, at) {
            out.push_str(&replace_text(&run[start..at], find, replace, opts, report, samples_left));
            out.push_str(&run[at..end]);
            start = end; i = end;
        } else { i = at + 1; }
    }
    out.push_str(&replace_text(&run[start..], find, replace, opts, report, samples_left));
}

pub fn replace_in_xhtml(src: &str, find: &str, replace: &str, opts: Options) -> (String, Report) {
    let mut report = Report::default();
    let mut samples_left = MAX_SAMPLES_PER_FILE;
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        if src.as_bytes()[i] == b'<' {
            let rest = &src[i..];
            let end = if rest.starts_with("<!--") { rest.find("-->").map(|p| i + p + 3).unwrap_or(src.len()) }
                else if rest.starts_with("<![CDATA[") { rest.find("]]>").map(|p| i + p + 3).unwrap_or(src.len()) }
                else { tag_end(src, i) };
            out.push_str(&src[i..end]);
            // Conteúdo de <script>/<style> não é texto do livro.
            let name: String = src[i + 1..end.min(src.len())].chars().take_while(|c| c.is_ascii_alphabetic()).collect::<String>().to_ascii_lowercase();
            let self_closed = src[i..end].trim_end_matches('>').ends_with('/');
            if (name == "script" || name == "style") && !self_closed && !src[i..end].starts_with("</") {
                let closing = format!("</{name}");
                let lower_rest = src[end..].to_ascii_lowercase();
                let stop = lower_rest.find(&closing).map(|p| end + p).unwrap_or(src.len());
                out.push_str(&src[end..stop]);
                i = stop;
                continue;
            }
            i = end;
            continue;
        }
        let next = src[i..].find('<').map(|p| i + p).unwrap_or(src.len());
        replace_xhtml_text(&src[i..next], find, replace, opts, &mut report, &mut samples_left, &mut out);
        i = next;
    }
    (out, report)
}

/// Markdown/Obsidian: só o texto corrido. `[[alvo]]` mantém o alvo; se o texto visível do link
/// continha o termo e não há `|`, vira `[[alvo|novo texto]]` (o link continua apontando para a mesma nota).
pub fn replace_in_markdown(src: &str, find: &str, replace: &str, opts: Options) -> (String, Report) {
    use regex::Regex;
    use std::sync::OnceLock;
    static TOKEN: OnceLock<Regex> = OnceLock::new();
    let token = TOKEN.get_or_init(|| Regex::new(r"\[\[([^\]\n]*)\]\]|`[^`\n]*`|\]\([^)\n]*\)|https?://[^\s)\]]+").expect("regex"));
    let mut report = Report::default();
    let mut samples_left = MAX_SAMPLES_PER_FILE;
    let mut out = String::with_capacity(src.len());
    let (mut in_front, mut fenced) = (false, false);
    for (n, line) in src.split_inclusive('\n').enumerate() {
        let content = line.trim_end_matches(['\r', '\n']);
        let ending = &line[content.len()..];
        if n == 0 && content == "---" { in_front = true; out.push_str(line); continue; }
        if in_front {
            if content == "---" { in_front = false; }
            let key = content.split(':').next().unwrap_or("").trim();
            if matches!(key, "tipo" | "estado" | "nivel" | "livro") || content == "---" { out.push_str(line); continue; }
        }
        if content.trim_start().starts_with("```") { fenced = !fenced; out.push_str(line); continue; }
        if fenced { out.push_str(line); continue; }
        let mut last = 0;
        let mut piece = String::new();
        for m in token.find_iter(content) {
            piece.push_str(&replace_text(&content[last..m.start()], find, replace, opts, &mut report, &mut samples_left));
            let text = m.as_str();
            if let Some(inner) = text.strip_prefix("[[").and_then(|t| t.strip_suffix("]]")) {
                match inner.split_once('|') {
                    Some((target, display)) => { piece.push_str("[["); piece.push_str(target); piece.push('|'); piece.push_str(&replace_text(display, find, replace, opts, &mut report, &mut samples_left)); piece.push_str("]]"); }
                    None if !inner.contains('/') && !inner.contains('#') => {
                        let mut local = Report::default();
                        let changed = replace_text(inner, find, replace, opts, &mut local, &mut samples_left);
                        if local.count > 0 { report.merge(&local, usize::MAX); piece.push_str(&format!("[[{inner}|{changed}]]")); } else { piece.push_str(text); }
                    }
                    None => piece.push_str(text),
                }
            } else { piece.push_str(text); }
            last = m.end();
        }
        piece.push_str(&replace_text(&content[last..], find, replace, opts, &mut report, &mut samples_left));
        out.push_str(&piece);
        out.push_str(ending);
    }
    (out, report)
}

/// Hash estável (FNV-1a 64) para detectar se um arquivo mudou desde a substituição.
pub fn fingerprint(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes { hash ^= *b as u64; hash = hash.wrapping_mul(0x100000001b3); }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const O: Options = Options { ignore_accents: false, preserve_case: true };

    fn x(src: &str, find: &str, rep: &str) -> (String, Report) { replace_in_xhtml(src, find, rep, O) }

    #[test]
    fn replaces_whole_words_and_keeps_the_case_of_each_occurrence() {
        let (out, r) = x("<p>Caio chegou. caio? CAIO! Caio.</p>", "Caio", "Jordan");
        assert_eq!(out, "<p>Jordan chegou. jordan? JORDAN! Jordan.</p>");
        assert_eq!((r.count, r.partial), (4, 0));
        assert_eq!(r.forms["Caio"], 2);
        assert_eq!(r.forms["caio"], 1);
    }

    #[test]
    fn partial_words_are_left_alone_but_reported() {
        let (out, r) = x("<p>Caiozinho e Caios viram Caio e o Acaio.</p>", "Caio", "Jordan");
        assert_eq!(out, "<p>Caiozinho e Caios viram Jordan e o Acaio.</p>");
        assert_eq!((r.count, r.partial), (1, 3));
        assert!(r.partial_words.contains_key("Caiozinho") && r.partial_words.contains_key("Acaio"));
    }

    #[test]
    fn tags_attributes_comments_entities_and_scripts_are_never_touched() {
        let src = "<p id=\"caio\" class=\"Caio\" title=\"Caio\"><a href=\"caio.xhtml\">Caio</a><!-- Caio --><style>.Caio{}</style><script>var Caio;</script>&Caio; Caio&nbsp;Caio</p>";
        let (out, r) = x(src, "Caio", "Jordan");
        assert_eq!(out, "<p id=\"caio\" class=\"Caio\" title=\"Caio\"><a href=\"caio.xhtml\">Jordan</a><!-- Caio --><style>.Caio{}</style><script>var Caio;</script>&Caio; Jordan&nbsp;Jordan</p>");
        assert_eq!(r.count, 3);
    }

    #[test]
    fn attribute_values_containing_gt_do_not_break_tag_detection() {
        let (out, r) = x("<p data-x=\"a>Caio\">Caio</p>", "Caio", "Jordan");
        assert_eq!(out, "<p data-x=\"a>Caio\">Jordan</p>");
        assert_eq!(r.count, 1);
    }

    #[test]
    fn accents_are_exact_unless_requested() {
        let (out, r) = replace_in_xhtml("<p>Íris, Iris e íris.</p>", "Iris", "Ana", O);
        assert_eq!(out, "<p>Íris, Ana e íris.</p>");
        assert_eq!(r.count, 1);
        let (out, r) = replace_in_xhtml("<p>Íris, Iris e íris.</p>", "Iris", "Ana", Options { ignore_accents: true, preserve_case: true });
        assert_eq!(out, "<p>Ana, Ana e ana.</p>");
        assert_eq!(r.count, 3);
    }

    #[test]
    fn multiword_terms_and_apostrophes() {
        let (out, _) = x("<p>Caio\n  Barbosa e Caio&nbsp;Barbosa; CAIO BARBOSA.</p>", "Caio Barbosa", "Jordan Barbosa");
        assert!(out.starts_with("<p>Jordan Barbosa e Caio&nbsp;Barbosa; JORDAN BARBOSA."), "{out}");
        // apóstrofo entre letras faz parte da palavra
        let (out, r) = x("<p>O Vel'shar e o Vel viram.</p>", "Vel", "Zed");
        assert_eq!(out, "<p>O Vel'shar e o Zed viram.</p>");
        assert_eq!((r.count, r.partial), (1, 1));
        let (out, r) = x("<p>O Vel'shar chegou.</p>", "Vel'shar", "Zed'kan");
        assert_eq!(out, "<p>O Zed'kan chegou.</p>");
        assert_eq!(r.count, 1);
    }

    #[test]
    fn case_options() {
        assert_eq!(adapt_case("caio", "Jordan", true), "jordan");
        assert_eq!(adapt_case("Caio", "jordan", true), "Jordan");
        assert_eq!(adapt_case("CAIO", "Jordan", true), "JORDAN");
        assert_eq!(adapt_case("cAio", "Jordan", true), "Jordan");
        assert_eq!(adapt_case("caio", "Jordan", false), "Jordan");
    }

    #[test]
    fn samples_show_context_and_the_replacement() {
        let (_, r) = x("<p>Ontem à noite o Caio disse que voltaria cedo.</p>", "Caio", "Jordan");
        assert_eq!(r.samples.len(), 1);
        assert_eq!(r.samples[0].matched, "Caio");
        assert_eq!(r.samples[0].replacement, "Jordan");
        assert!(r.samples[0].before.ends_with("o") && r.samples[0].after.starts_with("disse"));
    }

    #[test]
    fn nothing_found_returns_identical_text() {
        let src = "<p>Nada aqui &amp; ali.</p>";
        let (out, r) = x(src, "Caio", "Jordan");
        assert_eq!(out, src);
        assert_eq!(r.count, 0);
    }

    #[test]
    fn markdown_keeps_link_targets_code_urls_and_frontmatter_keys() {
        let src = "---\ntipo: personagem\naliases: [Caio, Cai]\n---\n# Caio Barbosa\n\nCaio viu [[Caio Barbosa]] e [[Fichas/Caio-Barbosa]] e [[Caio Barbosa|Caio]].\n`Caio` http://x.com/Caio [site](http://a/Caio) Caio\n```\nCaio\n```\n";
        let (out, r) = replace_in_markdown(src, "Caio", "Jordan", O);
        assert!(out.contains("aliases: [Jordan, Cai]"), "{out}");
        assert!(out.contains("# Jordan Barbosa"));
        assert!(out.contains("Jordan viu [[Caio Barbosa|Jordan Barbosa]] e [[Fichas/Caio-Barbosa]] e [[Caio Barbosa|Jordan]]."), "{out}");
        assert!(out.contains("`Caio` http://x.com/Caio [site](http://a/Caio) Jordan"));
        assert!(out.contains("```\nCaio\n```"));
        assert!(out.contains("tipo: personagem"));
        assert_eq!(r.count, 6);
    }

    #[test]
    fn crlf_is_preserved() {
        let (out, _) = replace_in_markdown("Caio\r\nvoltou\r\n", "Caio", "Jordan", O);
        assert_eq!(out, "Jordan\r\nvoltou\r\n");
    }

    #[test]
    fn fingerprint_is_stable_and_sensitive() {
        assert_eq!(fingerprint(b"abc"), fingerprint(b"abc"));
        assert_ne!(fingerprint(b"abc"), fingerprint(b"abd"));
    }
}
