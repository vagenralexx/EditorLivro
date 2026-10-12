//! Checagem determinística de fidelidade original × tradução, a partir do
//! checklist de `/revisar-traducao` (producao.md): parágrafos, diálogos,
//! expansão, glossário e preservação de linguagem crua. Opera no capítulo
//! inteiro, o que a torna robusta mesmo sem alinhamento parágrafo a parágrafo.

use crate::glossary::Glossary;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphFlag { pub index: usize, pub kind: &'static str, pub ratio: f32, pub original: String, pub translated: String }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TermCheck { pub term: String, pub original: usize, pub translated: usize, pub status: &'static str }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawCheck { pub raw: String, pub equivalents: Vec<String>, pub original: usize, pub translated: usize, pub status: &'static str }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FidelityReport {
    pub original_paragraphs: usize,
    pub translated_paragraphs: usize,
    pub original_dialogues: usize,
    pub translated_dialogues: usize,
    pub original_words: usize,
    pub translated_words: usize,
    /// Tradução ÷ original em palavras (EN→PT costuma ficar entre 1,05 e 1,30).
    pub expansion: f32,
    pub aligned: bool,
    pub flags: Vec<ParagraphFlag>,
    pub terms: Vec<TermCheck>,
    pub raw_language: Vec<RawCheck>,
    pub notes: Vec<String>,
}

fn words(text: &str) -> usize { text.split_whitespace().filter(|w| w.chars().any(char::is_alphanumeric)).count() }

fn is_dialogue(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with('—') || t.starts_with('–') || t.starts_with("- ") || t.starts_with('“') || t.starts_with('"') || t.starts_with('«') || t.starts_with('‘')
}

fn snippet(text: &str) -> String { text.chars().take(90).collect() }

fn is_letter(c: char) -> bool { c.is_alphabetic() }

/// Conta `needle` (já em minúsculas) em `haystack` (minúsculas) com fronteira de palavra.
/// Com `prefix`, aceita qualquer sufixo de letras (fuck → fucking, foder → fodendo).
fn count(haystack: &str, needle: &str, prefix: bool) -> usize {
    if needle.is_empty() { return 0; }
    haystack.match_indices(needle).filter(|(start, _)| {
        let before_ok = haystack[..*start].chars().next_back().map(|c| !is_letter(c)).unwrap_or(true);
        let after_ok = prefix || haystack[start + needle.len()..].chars().next().map(|c| !is_letter(c)).unwrap_or(true);
        before_ok && after_ok
    }).count()
}

/// Radical para casar conjugações: remove 2 letras finais de palavras longas.
fn stem(word: &str) -> String {
    let chars: Vec<char> = word.to_lowercase().chars().collect();
    if chars.len() >= 6 { chars[..chars.len() - 2].iter().collect() } else if chars.len() >= 4 { chars[..chars.len() - 1].iter().collect() } else { chars.iter().collect() }
}

pub fn compare(original: &[String], translated: &[String], glossary: &Glossary) -> FidelityReport {
    let original_text = original.join("\n").to_lowercase();
    let translated_text = translated.join("\n").to_lowercase();
    let (original_words, translated_words) = (original.iter().map(|l| words(l)).sum::<usize>(), translated.iter().map(|l| words(l)).sum::<usize>());
    let expansion = if original_words > 0 { translated_words as f32 / original_words as f32 } else { 0.0 };
    let aligned = !original.is_empty() && original.len() == translated.len();
    let mut notes: Vec<String> = Vec::new();

    if original_words >= 150 {
        if expansion < 0.85 { notes.push(format!("A tradução tem {:.0}% das palavras do original: bem abaixo do esperado (EN→PT costuma crescer 5–30%). Possível resumo ou omissão.", expansion * 100.0)); }
        if expansion > 1.5 { notes.push(format!("A tradução tem {:.0}% das palavras do original: bem acima do esperado. Possível adição não autorizada.", expansion * 100.0)); }
    }
    let (od, td) = (original.iter().filter(|l| is_dialogue(l)).count(), translated.iter().filter(|l| is_dialogue(l)).count());
    if od != td { notes.push(format!("Diálogos: original {od}, tradução {td}. A skill exige o mesmo número de falas.")); }
    if original.len() != translated.len() { notes.push(format!("Parágrafos: original {}, tradução {} (diferença {:+}). Parágrafos fundidos/divididos ou omitidos; a checagem por parágrafo fica desativada.", original.len(), translated.len(), translated.len() as i64 - original.len() as i64)); }

    let mut flags = Vec::new();
    if aligned {
        for (index, (o, t)) in original.iter().zip(translated).enumerate() {
            let (ow, tw) = (words(o), words(t));
            if ow < 6 { continue; }
            let ratio = tw as f32 / ow as f32;
            if ratio < 0.5 { flags.push(ParagraphFlag { index, kind: "omissao", ratio, original: snippet(o), translated: snippet(t) }); }
            else if ratio > 2.2 { flags.push(ParagraphFlag { index, kind: "adicao", ratio, original: snippet(o), translated: snippet(t) }); }
        }
    }

    let mut terms: Vec<TermCheck> = Vec::new();
    for term in &glossary.nao_traduzir {
        let needle = term.to_lowercase();
        let (o, t) = (count(&original_text, &needle, false), count(&translated_text, &needle, false));
        if o == 0 && t == 0 { continue; }
        let status = if o > 0 && t == 0 { "ausente" } else if t < o { "reduzido" } else { "ok" };
        terms.push(TermCheck { term: term.clone(), original: o, translated: t, status });
    }
    for (from, to) in &glossary.traducoes_fixas {
        let (o, t) = (count(&original_text, &from.to_lowercase(), false), count(&translated_text, &to.to_lowercase(), false));
        if o == 0 && t == 0 { continue; }
        terms.push(TermCheck { term: format!("{from} → {to}"), original: o, translated: t, status: if o > 0 && t < o { "reduzido" } else { "ok" } });
    }

    let mut raw_language: Vec<RawCheck> = Vec::new();
    for (raw, equivalents) in &glossary.palavroes {
        let o = count(&original_text, &stem(raw), true);
        if o == 0 { continue; }
        let t: usize = equivalents.iter().map(|e| count(&translated_text, &stem(e), true)).sum();
        // Conta equivalentes “a mais” que possam ter vindo de outro termo cru: só sinalizamos queda clara.
        let status = if t == 0 { "suavizado" } else if (t as f32) < o as f32 * 0.7 { "possivelmente suavizado" } else { "ok" };
        raw_language.push(RawCheck { raw: raw.clone(), equivalents: equivalents.clone(), original: o, translated: t, status });
    }

    FidelityReport { original_paragraphs: original.len(), translated_paragraphs: translated.len(), original_dialogues: od, translated_dialogues: td, original_words, translated_words, expansion, aligned, flags, terms, raw_language, notes }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &[&str]) -> Vec<String> { text.iter().map(|s| s.to_string()).collect() }

    fn glossary() -> Glossary {
        let mut g = Glossary::default();
        g.nao_traduzir = vec!["Vhraress".into(), "rwy'n la".into()];
        g.traducoes_fixas.insert("my heart".into(), "meu coração".into());
        g.palavroes.insert("fuck".into(), vec!["foder".into(), "porra".into(), "caralho".into()]);
        g.palavroes.insert("damn".into(), vec!["porra".into()]);
        g
    }

    #[test]
    fn counts_terms_with_word_boundaries_and_stems() {
        assert_eq!(count("fuck fucking fucked unfuck", "fuck", true), 3);
        assert_eq!(count("kix e kix.", "kix", false), 2);
        assert_eq!(count("kixes", "kix", false), 0);
        assert_eq!(stem("foder"), "fode");
        assert_eq!(stem("porra"), "porr");
        assert_eq!(count("ela fodeu, fodendo e fode", &stem("foder"), true), 3);
    }

    #[test]
    fn flags_glossary_terms_that_were_translated_or_lost() {
        let o = lines(&["“Rwy'n la,” said Vhraress. “Vhraress, my heart.”"]);
        let t = lines(&["— Eu te amo, disse o guerreiro. — Meu coração."]);
        let r = compare(&o, &t, &glossary());
        let vhraress = r.terms.iter().find(|x| x.term == "Vhraress").unwrap();
        assert_eq!((vhraress.original, vhraress.translated, vhraress.status), (2, 0, "ausente"));
        assert!(r.terms.iter().any(|x| x.term == "rwy'n la" && x.status == "ausente"));
        let fixed = r.terms.iter().find(|x| x.term.starts_with("my heart")).unwrap();
        assert_eq!((fixed.original, fixed.translated, fixed.status), (1, 1, "ok"));
    }

    #[test]
    fn detects_softened_raw_language() {
        let o = lines(&["Fuck, he said. Damn it, she said. Fucking hell, fuck."]);
        let soft = lines(&["Droga, disse ele. Maldição, disse ela. Inferno."]);
        let r = compare(&o, &soft, &glossary());
        let fuck = r.raw_language.iter().find(|x| x.raw == "fuck").unwrap();
        assert_eq!((fuck.original, fuck.translated, fuck.status), (3, 0, "suavizado"));
        let faithful = lines(&["Porra, disse ele. Porra, disse ela. Caralho, que foda."]);
        let ok = compare(&o, &faithful, &glossary());
        assert!(ok.raw_language.iter().all(|x| x.status == "ok"), "{:?}", ok.raw_language);
    }

    #[test]
    fn flags_omissions_and_additions_only_when_paragraphs_align() {
        let original = lines(&["She walked slowly across the empty room and looked at the old window.", "He said nothing at all for a very long time after that.", "Short."]);
        let translated = lines(&["Ela andou.", "Ele não disse nada por muito tempo depois disso, e depois caminhou até a porta, abriu, olhou o corredor vazio, suspirou fundo, lembrou do que prometera à irmã na primavera passada e saiu sem olhar para trás.", "Curto."]);
        let r = compare(&original, &translated, &Glossary::default());
        assert!(r.aligned);
        assert_eq!(r.flags.iter().map(|f| (f.index, f.kind)).collect::<Vec<_>>(), vec![(0, "omissao"), (1, "adicao")]);
        let broken = compare(&original, &translated[..2], &Glossary::default());
        assert!(!broken.aligned && broken.flags.is_empty());
        assert!(broken.notes.iter().any(|n| n.contains("Parágrafos: original 3, tradução 2")));
    }

    #[test]
    fn reports_dialogue_count_and_expansion() {
        let original: Vec<String> = (0..30).map(|i| format!("“Line number {i} of the dialogue goes on and on for several words,” he said.")).collect();
        let summarized: Vec<String> = (0..30).map(|i| format!("— Fala {i}.")).collect();
        let r = compare(&original, &summarized, &Glossary::default());
        assert!(r.expansion < 0.5);
        assert!(r.notes.iter().any(|n| n.contains("Possível resumo")));
        let fewer = compare(&original, &summarized[..25], &Glossary::default());
        assert!(fewer.notes.iter().any(|n| n.contains("Diálogos: original 30, tradução 25")));
    }

    #[test]
    fn empty_inputs_are_safe() {
        let r = compare(&[], &[], &glossary());
        assert_eq!((r.original_words, r.expansion), (0, 0.0));
        assert!(!r.aligned);
    }
}
