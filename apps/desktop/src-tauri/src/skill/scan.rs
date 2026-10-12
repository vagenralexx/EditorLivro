//! Diagnóstico instantâneo (sem IA): métricas e varredura dos padrões da skill
//! que podem ser detectados por regras. Tudo aqui são *candidatos* para o
//! editor humano confirmar, nunca um veredito automático.

use super::genre::Genre;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ScanOptions {
    pub genre: Genre,
    /// O texto é uma tradução EN→PT-BR: ativa os alertas de suavização e as regras de fluidez.
    pub translation_mode: bool,
    /// Manuscrito com cenas adultas: acrescenta alertas de eufemismo clínico.
    pub adult_content: bool,
    /// O trecho começa no início de um capítulo (liga a checagem de abertura).
    pub is_opening: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternHit {
    pub id: u8,
    pub name: &'static str,
    pub count: usize,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCount { pub tag: String, pub count: usize }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Note { pub kind: &'static str, pub message: String, pub example: Option<String> }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FluencySuggestion { pub before: String, pub after: String, pub reason: String }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub words: usize,
    pub sentences: usize,
    pub paragraphs: usize,
    pub avg_sentence_words: f32,
    /// Coeficiente de variação do tamanho das frases (ritmo). Perto de 0 = monótono.
    pub sentence_variation: f32,
    pub has_acceleration: bool,
    pub has_deceleration: bool,
    pub dialogue_percent: f32,
    pub dialogue_target: Option<(u32, u32)>,
    pub tags: Vec<TagCount>,
    pub patterns: Vec<PatternHit>,
    pub pattern_total: usize,
    /// Ocorrências por 3.000 palavras (mesma base usada pela skill).
    pub patterns_per_3k: f32,
    pub patterns_max: u32,
    /// Quantos dos 20 padrões a varredura consegue procurar (o resto exige leitura/IA).
    pub patterns_covered: usize,
    pub notes: Vec<Note>,
    pub fluency: Vec<FluencySuggestion>,
}

/// Os 20 padrões da skill (analise.md). `detectable` indica se há regra aqui.
pub const PATTERNS: [(u8, &str, bool); 20] = [
    (1, "Gerúndio de abertura de parágrafo", true),
    (2, "Emoção nomeada antes de demonstrada", true),
    (3, "Observação que não observa (“notou que”)", true),
    (4, "Eco temático em cada detalhe", false),
    (5, "Resolução emocional dentro do capítulo", false),
    (6, "Diálogo que expõe o personagem diretamente", false),
    (7, "Símile analítica que explica", false),
    (8, "Estrutura “não X, mas Y”", true),
    (9, "Adjetivos duplos desnecessários", false),
    (10, "Precisão-flex desnecessária", false),
    (11, "Símile com extensão explanatória (grave; candidato)", true),
    (12, "Flashback motivado demais", false),
    (13, "Compostura pós-crise imediata", false),
    (14, "Paralelo semântico triplo", false),
    (15, "Verbo figurado (dançar/flutuar/sussurrar)", true),
    (16, "Contexto excessivo em diálogo de ação", false),
    (17, "Diálogo perfeitamente articulado sob pressão", false),
    (18, "Resolução moral limpa", false),
    (19, "Pensamento em itálico que resume o tema", false),
    (20, "Ambiente como espelho emocional mecânico", false),
];

struct Rules {
    emotion_named: Regex,
    observes: Regex,
    not_but: Regex,
    simile: Regex,
    figurative: Regex,
    tags: Regex,
    opening_weather: Regex,
    opening_waking: Regex,
    opening_backstory: Regex,
    opening_abstract: Regex,
    soften: Vec<(Regex, &'static str)>,
    soften_adult: Vec<(Regex, &'static str)>,
    fluency: Vec<(Regex, String, String)>,
}

fn rules() -> &'static Rules {
    static RULES: OnceLock<Rules> = OnceLock::new();
    RULES.get_or_init(|| {
        let re = |pattern: &str| Regex::new(pattern).unwrap_or_else(|error| panic!("regex inválida {pattern}: {error}"));
        let fluency = include_str!("../../skill/fluidez.tsv").lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()).filter_map(|line| {
            let mut parts = line.split('\t');
            let (category, pattern, replacement) = (parts.next()?, parts.next()?, parts.next()?);
            Some((re(pattern), replacement.to_string(), category.to_string()))
        }).collect();
        Rules {
            emotion_named: re(r"(?i)\b(?:sentiu|sentia|sentira|sentindo)\s+(?:uma?\s+)?(?:onda|pontada|fisgada|enxurrada|nó|aperto|frio|calafrio|arrepio|descarga|pico)\s+de\b|(?i)\b(?:sentiu|sentia)\s+(?:medo|raiva|tristeza|alívio|culpa|vergonha|angústia|ansiedade|pânico|desespero|saudade|ódio|ciúme)\b"),
            observes: re(r"(?i)\b(?:notou|percebeu|reparou|observou|notando|percebendo)\s+que\b|(?i)\bse\s+deu\s+conta\s+de\s+que\b"),
            not_but: re(r"(?i)\bnão\b[^.!?\n—]{1,70}?,\s*(?:mas|e\s+sim)\b"),
            simile: re(r"(?i)\bcomo\s+(?:alguém|quem)\b"),
            figurative: re(r"(?i)\b(?:danç(?:a|ava|avam|am|ou|aram|ando)|flutu(?:a|ava|avam|am|ou|aram|ando)|sussurr(?:a|ava|avam|am|ou|aram|ando))\b"),
            tags: re(r"(?i)\b(disse|falou|perguntou|respondeu|sussurrou|gritou|murmurou|exclamou|retrucou|replicou|completou|continuou|acrescentou|indagou|ordenou|rosnou|bufou|riu|suspirou|resmungou|provocou|ofegou|gemeu)\b"),
            opening_weather: re(r"(?i)\b(?:chovia|chuva|amanhecia|amanheceu|nublado|o\s+sol\b|o\s+céu\b|a\s+manhã\b|o\s+clima\b|fazia\s+(?:frio|calor))"),
            opening_waking: re(r"(?i)\b(?:acordou|despertou|despertador|levantou\s+da\s+cama|escovou\s+os\s+dentes|tomou\s+café)\b"),
            opening_backstory: re(r"(?i)\b(?:anos\s+atrás|havia\s+sido|tinha\s+sido|naquela\s+época|desde\s+criança)\b"),
            opening_abstract: re(r"(?i)\b(?:a\s+vida\s+(?:é|era)|o\s+amor\s+(?:é|era)|o\s+tempo\s+(?:é|era)|todo\s+mundo\s+sabe)\b"),
            soften: vec![
                (re(r"(?i)\bmaldit[oa]s?\b"), "“maldito” costuma suavizar damn/fuck"),
                (re(r"(?i)\bdroga\b"), "“droga” costuma suavizar damn/shit"),
                (re(r"(?i)\bcaramba\b"), "“caramba” costuma suavizar damn/hell"),
                (re(r"(?i)\bdiabos\b"), "“diabos” costuma suavizar hell/fuck"),
                (re(r"(?i)\bmeu\s+deus\b"), "“meu Deus” costuma suavizar holy shit/oh fuck"),
                (re(r"(?i)\bbruxa\b"), "“bruxa” costuma suavizar bitch"),
                (re(r"(?i)\bnem\s+um\s+pouco\b"), "“nem um pouco” costuma suavizar not at all (porra nenhuma)"),
                (re(r"(?i)\bperder\s+o\s+juízo\b"), "“perder o juízo” costuma suavizar lose her damn mind"),
            ],
            soften_adult: vec![
                (re(r"(?i)\bmembro\b"), "“membro” é antiquado/clínico em cena sexual"),
                (re(r"(?i)\bpênis\b"), "“pênis” é clínico em cena sexual"),
                (re(r"(?i)\bórgão(?:s)?\s+genita(?:l|is)\b"), "“órgão genital” é clínico em cena sexual"),
                (re(r"(?i)\b(?:o|no|do|ao)\s+sexo\s+(?:dela|dele)\b"), "“sexo” como eufemismo de órgão quebra o registro"),
                (re(r"(?i)\bintimidade\s+(?:dela|dele)\b"), "“intimidade” como eufemismo é vago"),
            ],
            fluency,
        }
    })
}

const NOT_GERUNDS: [&str; 14] = ["mundo", "fundo", "quando", "segundo", "profundo", "redondo", "oriundo", "moribundo", "vagabundo", "fecundo", "jucundo", "rotundo", "nauseabundo", "tremebundo"];
const ABBREVIATIONS: [&str; 12] = ["sr", "sra", "srta", "dr", "dra", "prof", "profa", "av", "etc", "ex", "nº", "pág"];
const BASIC_TAGS: [&str; 4] = ["disse", "falou", "perguntou", "respondeu"];

fn is_dialogue_paragraph(paragraph: &str) -> bool {
    let trimmed = paragraph.trim_start();
    trimmed.starts_with('—') || trimmed.starts_with('–') || trimmed.starts_with("- ") || trimmed.starts_with('“') || trimmed.starts_with('«') || trimmed.starts_with('"')
}

fn count_words(text: &str) -> usize {
    text.split_whitespace().filter(|word| word.chars().any(char::is_alphanumeric)).count()
}

fn split_sentences(paragraph: &str) -> Vec<String> {
    let chars: Vec<char> = paragraph.chars().collect();
    let mut sentences = Vec::new();
    let mut current = String::new();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        current.push(ch);
        if matches!(ch, '.' | '!' | '?' | '…') {
            while index + 1 < chars.len() && matches!(chars[index + 1], '.' | '!' | '?' | '…' | '”' | '"' | '»' | ')') {
                index += 1;
                current.push(chars[index]);
            }
            let at_end = index + 1 >= chars.len();
            let next_is_space = !at_end && chars[index + 1].is_whitespace();
            let last_word = current.trim_end_matches(['.', '!', '?', '…']).rsplit(char::is_whitespace).next().unwrap_or("").to_lowercase();
            let abbreviation = ch == '.' && ABBREVIATIONS.contains(&last_word.as_str());
            if (at_end || next_is_space) && !abbreviation {
                if count_words(&current) > 0 { sentences.push(current.trim().to_string()); }
                current.clear();
            }
        }
        index += 1;
    }
    if count_words(&current) > 0 { sentences.push(current.trim().to_string()); }
    sentences
}

fn context(text: &str, start: usize, end: usize) -> String {
    let before: String = text[..start].chars().rev().take(20).collect::<Vec<_>>().into_iter().rev().collect();
    let after: String = text[end..].chars().take(24).collect();
    let snippet = format!("{}{}{}", before, &text[start..end], after).replace('\n', " ");
    snippet.trim().to_string()
}

struct Collector { count: usize, examples: Vec<String> }
impl Collector {
    fn new() -> Self { Collector { count: 0, examples: Vec::new() } }
    fn add(&mut self, text: &str, start: usize, end: usize) {
        self.count += 1;
        if self.examples.len() < 3 { self.examples.push(context(text, start, end)); }
    }
}

fn is_gerund_opening(paragraph: &str) -> Option<String> {
    let trimmed = paragraph.trim_start();
    if is_dialogue_paragraph(trimmed) { return None; }
    let word: String = trimmed.chars().take_while(|c| c.is_alphabetic()).collect();
    let lower = word.to_lowercase();
    (lower.len() >= 5 && lower.ends_with("ndo") && !NOT_GERUNDS.contains(&lower.as_str())).then(|| trimmed.chars().take(50).collect())
}

pub fn scan(text: &str, options: &ScanOptions) -> ScanReport {
    let rules = rules();
    let paragraphs: Vec<&str> = text.lines().map(str::trim).filter(|p| count_words(p) > 0).collect();
    let words = count_words(text);

    // Frases e ritmo.
    let mut lengths: Vec<usize> = Vec::new();
    for paragraph in &paragraphs {
        for sentence in split_sentences(paragraph) { lengths.push(count_words(&sentence)); }
    }
    let sentences = lengths.len();
    let mean = if sentences > 0 { lengths.iter().sum::<usize>() as f32 / sentences as f32 } else { 0.0 };
    let variance = if sentences > 1 { lengths.iter().map(|&l| (l as f32 - mean).powi(2)).sum::<f32>() / sentences as f32 } else { 0.0 };
    let variation = if mean > 0.0 { variance.sqrt() / mean } else { 0.0 };
    let run = |predicate: &dyn Fn(usize) -> bool, min: usize| lengths.windows(min).any(|window| window.iter().all(|&l| predicate(l)));
    let has_acceleration = run(&|l| l <= 8, 3);
    let has_deceleration = run(&|l| l >= 25, 1);

    // Diálogo e tags.
    let dialogue_words: usize = paragraphs.iter().filter(|p| is_dialogue_paragraph(p)).map(|p| count_words(p)).sum();
    let dialogue_percent = if words > 0 { dialogue_words as f32 * 100.0 / words as f32 } else { 0.0 };
    let mut tag_counts: BTreeMap<String, usize> = BTreeMap::new();
    for paragraph in paragraphs.iter().filter(|p| is_dialogue_paragraph(p)) {
        for found in rules.tags.find_iter(paragraph) { *tag_counts.entry(found.as_str().to_lowercase()).or_default() += 1; }
    }
    let mut tags: Vec<TagCount> = tag_counts.into_iter().map(|(tag, count)| TagCount { tag, count }).collect();
    tags.sort_by(|a, b| b.count.cmp(&a.count).then(a.tag.cmp(&b.tag)));
    let tag_total: usize = tags.iter().map(|t| t.count).sum();

    // Padrões anti-IA detectáveis.
    let mut collectors: BTreeMap<u8, Collector> = BTreeMap::new();
    for paragraph in &paragraphs {
        if let Some(example) = is_gerund_opening(paragraph) {
            let c = collectors.entry(1).or_insert_with(Collector::new);
            c.count += 1;
            if c.examples.len() < 3 { c.examples.push(example); }
        }
    }
    let mut run_regex = |id: u8, regex: &Regex| {
        for found in regex.find_iter(text) { collectors.entry(id).or_insert_with(Collector::new).add(text, found.start(), found.end()); }
    };
    run_regex(2, &rules.emotion_named);
    run_regex(3, &rules.observes);
    run_regex(8, &rules.not_but);
    run_regex(11, &rules.simile);
    run_regex(15, &rules.figurative);
    let patterns: Vec<PatternHit> = collectors.into_iter().filter(|(_, c)| c.count > 0).map(|(id, c)| {
        let name = PATTERNS.iter().find(|p| p.0 == id).map(|p| p.1).unwrap_or("");
        PatternHit { id, name, count: c.count, examples: c.examples }
    }).collect();
    let pattern_total: usize = patterns.iter().map(|p| p.count).sum();
    let per_3k = if words > 0 { pattern_total as f32 * 3000.0 / words as f32 } else { 0.0 };

    // Notas editoriais (ritmo, diálogo, abertura, suavização).
    let mut notes: Vec<Note> = Vec::new();
    if let (Some((low, high)), true) = (options.genre.dialogue_range(), words >= 150) {
        if dialogue_percent < low as f32 { notes.push(Note { kind: "dialogo", message: format!("Diálogo em {:.0}%, abaixo da faixa de {} ({low}–{high}%).", dialogue_percent, options.genre.label()), example: None }); }
        if dialogue_percent > high as f32 { notes.push(Note { kind: "dialogo", message: format!("Diálogo em {:.0}%, acima da faixa de {} ({low}–{high}%).", dialogue_percent, options.genre.label()), example: None }); }
    }
    if tag_total >= 5 {
        if let Some(top) = tags.first() {
            let share = top.count as f32 / tag_total as f32;
            if share > 0.4 { notes.push(Note { kind: "dialogo", message: format!("A tag “{}” responde por {:.0}% das marcações de fala (acima de 40%).", top.tag, share * 100.0), example: None }); }
        }
        let creative: usize = tags.iter().filter(|t| !BASIC_TAGS.contains(&t.tag.as_str())).map(|t| t.count).sum();
        if creative as f32 / tag_total as f32 > 0.5 { notes.push(Note { kind: "dialogo", message: "Mais da metade das tags de fala são verbos “criativos” (rosnou, bufou…); a skill recomenda “disse” e ações específicas.".to_string(), example: None }); }
    }
    if let (Some(simile), true) = (patterns.iter().find(|p| p.id == 11), words >= 300) {
        let per_3k_11 = simile.count as f32 * 3000.0 / words as f32;
        if per_3k_11 > 4.0 {
            notes.push(Note { kind: "anti-ia", message: format!("{} candidatos a símile com extensão explanatória (#11, grave): {:.1} por 3.000 palavras; acima de 4 a skill reprova o capítulo. Confirme se a imagem é explicada logo depois.", simile.count, per_3k_11), example: simile.examples.first().cloned() });
        } else if per_3k_11 > options.genre.simile_extension_max() as f32 {
            notes.push(Note { kind: "anti-ia", message: format!("{} candidatos a símile com extensão (#11): {:.1} por 3.000 palavras, acima da meta de {} para {}.", simile.count, per_3k_11, options.genre.simile_extension_max(), options.genre.label()), example: simile.examples.first().cloned() });
        }
    }
    if sentences >= 12 {
        if variation < 0.35 { notes.push(Note { kind: "ritmo", message: format!("Frases de tamanho muito parecido (variação {:.2}): ritmo monótono.", variation), example: None }); }
        if !has_acceleration || !has_deceleration { notes.push(Note { kind: "ritmo", message: format!("Faltam {}: a skill pede ao menos uma aceleração e uma desaceleração por capítulo (heurística por tamanho de frase).", match (has_acceleration, has_deceleration) { (false, false) => "aceleração e desaceleração", (false, true) => "aceleração (sequência de frases curtas)", _ => "desaceleração (frase longa)" }), example: None }); }
    }
    if options.is_opening {
        let opening: String = paragraphs.iter().flat_map(|p| split_sentences(p)).take(3).collect::<Vec<_>>().join(" ");
        for (regex, message) in [
            (&rules.opening_weather, "A abertura começa com tempo/clima sem tensão (proibido nas primeiras frases)."),
            (&rules.opening_waking, "A abertura mostra o personagem acordando/em rotina matinal (proibido nas primeiras frases)."),
            (&rules.opening_backstory, "A abertura traz backstory/contexto (proibido nas primeiras frases)."),
            (&rules.opening_abstract, "A abertura é reflexão abstrata (proibido nas primeiras frases)."),
        ] {
            if let Some(found) = regex.find(&opening) { notes.push(Note { kind: "hook", message: message.to_string(), example: Some(context(&opening, found.start(), found.end())) }); }
        }
    }
    let mut soften_lists: Vec<&Vec<(Regex, &str)>> = Vec::new();
    if options.translation_mode { soften_lists.push(&rules.soften); }
    if options.translation_mode && options.adult_content { soften_lists.push(&rules.soften_adult); }
    if options.adult_content && !options.translation_mode { soften_lists.push(&rules.soften_adult); }
    for list in soften_lists {
        for (regex, message) in list.iter() {
            if let Some(found) = regex.find(text) {
                let count = regex.find_iter(text).count();
                notes.push(Note { kind: "suavizacao", message: format!("{message} ({count}×; confira com o original)."), example: Some(context(text, found.start(), found.end())) });
            }
        }
    }

    // Regras de fluidez (tradução): viram sugestões antes/depois para aprovação.
    let mut fluency: Vec<FluencySuggestion> = Vec::new();
    if options.translation_mode {
        for (regex, replacement, category) in &rules.fluency {
            for found in regex.find_iter(text) {
                if fluency.len() >= 25 { break; }
                if fluency.iter().any(|s| s.before == found.as_str()) { continue; }
                fluency.push(FluencySuggestion { before: found.as_str().to_string(), after: replacement.clone(), reason: format!("Fluidez PT-BR ({}): construção literal do inglês.", category.to_lowercase().replace('_', " ")) });
            }
        }
    }

    ScanReport {
        words, sentences, paragraphs: paragraphs.len(),
        avg_sentence_words: mean, sentence_variation: variation, has_acceleration, has_deceleration,
        dialogue_percent, dialogue_target: options.genre.dialogue_range(),
        tags, patterns, pattern_total, patterns_per_3k: per_3k, patterns_max: options.genre.anti_ia_max(),
        patterns_covered: PATTERNS.iter().filter(|p| p.2).count(),
        notes, fluency,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(genre: Genre) -> ScanOptions { ScanOptions { genre, ..ScanOptions::default() } }

    #[test]
    fn counts_words_sentences_and_handles_abbreviations() {
        let r = scan("O Sr. Silva chegou. Ele sorriu!\nEla não respondeu...", &opts(Genre::Comercial));
        assert_eq!(r.paragraphs, 2);
        assert_eq!(r.sentences, 3, "“Sr.” não encerra a frase");
        assert_eq!(r.words, 9);
    }

    #[test]
    fn measures_dialogue_share_and_tag_overuse() {
        let text = "— Vem aqui, disse Ana.\n— Não vou, disse Beto.\n— Vai sim, disse Ana.\n— Então tá, disse Beto.\n— Pronto, disse Ana.\nEla guardou o papel na gaveta e saiu devagar pela porta dos fundos da casa.";
        let r = scan(text, &opts(Genre::Thriller));
        assert!(r.dialogue_percent > 50.0 && r.dialogue_percent < 80.0, "{}", r.dialogue_percent);
        assert_eq!(r.tags[0].tag, "disse");
        assert_eq!(r.tags[0].count, 5);
        assert!(r.notes.iter().any(|n| n.message.contains("“disse”")), "{:?}", r.notes);
    }

    #[test]
    fn detects_detectable_anti_ai_patterns_with_examples() {
        let text = "Virando-se para a janela, ela notou que a rua estava vazia.\nEla sentiu uma onda de medo.\nNão era raiva, mas cansaço.\nAbriu a porta como alguém que estuda uma doença.\nAs memórias dançavam na sua cabeça.\nQuando chegou, o mundo era outro.";
        let r = scan(text, &opts(Genre::Literaria));
        let ids: Vec<u8> = r.patterns.iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![1, 2, 3, 8, 11, 15], "{:?}", r.patterns.iter().map(|p| (p.id, p.count)).collect::<Vec<_>>());
        assert!(r.patterns.iter().all(|p| p.count == 1 && !p.examples.is_empty()));
        assert_eq!(r.patterns_covered, 6, "detectáveis por regra: 1, 2, 3, 8, 11 e 15");
        assert_eq!(r.patterns_max, 3);
        assert!(r.patterns_per_3k > 0.0);
    }

    #[test]
    fn gerund_opening_ignores_non_gerunds_and_dialogue() {
        assert!(is_gerund_opening("Olhando o relógio, ele saiu.").is_some());
        assert!(is_gerund_opening("Quando chegou, riu.").is_none());
        assert!(is_gerund_opening("Segundo ele, nada.").is_none());
        assert!(is_gerund_opening("— Voltando já.").is_none());
    }

    #[test]
    fn opening_checks_only_run_for_chapter_openings() {
        let text = "Chovia na cidade naquela manhã cinzenta. Maria acordou cedo e olhou o telhado. A vida era assim, repetitiva.";
        let off = scan(text, &opts(Genre::Literaria));
        assert!(off.notes.iter().all(|n| n.kind != "hook"));
        let on = scan(text, &ScanOptions { is_opening: true, ..opts(Genre::Literaria) });
        let kinds: Vec<&str> = on.notes.iter().filter(|n| n.kind == "hook").map(|n| n.message.as_str()).collect();
        assert_eq!(kinds.len(), 3, "{kinds:?}");
    }

    #[test]
    fn translation_mode_flags_softening_and_proposes_fluency_fixes() {
        let text = "— Droga, ela disse, alcançando a xícara. Então ele arqueou uma sobrancelha.";
        let plain = scan(text, &opts(Genre::Romance));
        assert!(plain.notes.iter().all(|n| n.kind != "suavizacao") && plain.fluency.is_empty());
        let r = scan(text, &ScanOptions { translation_mode: true, ..opts(Genre::Romance) });
        assert!(r.notes.iter().any(|n| n.kind == "suavizacao" && n.message.contains("droga")));
        let fix = r.fluency.iter().find(|s| s.before == "arqueou uma sobrancelha").expect("regra da skill");
        assert_eq!(fix.after, "ergueu uma sobrancelha");
        assert!(r.fluency.iter().any(|s| s.before == "alcançando" && s.after == "estendendo a mão"));
    }

    #[test]
    fn adult_softening_alerts_require_the_flag() {
        let text = "Ele tocou o membro dele com cuidado.";
        assert!(scan(text, &opts(Genre::Romance)).notes.is_empty());
        let r = scan(text, &ScanOptions { adult_content: true, ..opts(Genre::Romance) });
        assert!(r.notes.iter().any(|n| n.kind == "suavizacao"));
    }

    #[test]
    fn many_similes_trigger_the_grave_pattern_note() {
        let text = (0..6).map(|i| format!("Ele abriu a porta {i} como alguém que estuda uma doença rara. {}", "A casa estava em silêncio e nada se movia no corredor escuro. ".repeat(8))).collect::<String>();
        let r = scan(&text, &opts(Genre::Literaria));
        assert!(r.notes.iter().any(|n| n.kind == "anti-ia" && n.message.contains("#11")), "{:?}", r.notes);
    }

    #[test]
    fn rhythm_flags_monotony() {
        let text = (0..14).map(|i| format!("Ele andou pela rua número {i} agora.")).collect::<Vec<_>>().join(" ");
        let r = scan(&text, &opts(Genre::Comercial));
        assert!(r.sentence_variation < 0.1);
        assert!(r.notes.iter().any(|n| n.kind == "ritmo" && n.message.contains("monótono")));
    }

    #[test]
    fn fluency_rules_load_from_the_skill_file() {
        assert!(rules().fluency.len() >= 60);
    }

    #[test]
    fn empty_text_is_safe() {
        let r = scan("", &ScanOptions::default());
        assert_eq!((r.words, r.sentences, r.pattern_total), (0, 0, 0));
        assert_eq!(r.patterns_per_3k, 0.0);
    }
}
