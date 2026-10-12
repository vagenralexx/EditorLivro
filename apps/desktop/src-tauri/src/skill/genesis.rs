//! Genesis Score V3.7 (scores.md): a IA atribui as 7 notas com evidência e o
//! aplicativo faz a conta, para que o resultado seja determinístico.
//!   Floor = menor nota · Média = soma ÷ 7 · Genesis = Floor × 0,4 + Média × 0,6

use super::genre::Genre;
use serde::{Deserialize, Serialize};

pub const DIMENSIONS: [&str; 7] = ["Originalidade", "Tema", "Personagens", "Prosa & Voz", "Ritmo & Coerência", "Emoção", "Critério do gênero"];

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawScore {
    #[serde(default)]
    pub dimension: String,
    #[serde(default)]
    pub score: f32,
    #[serde(default)]
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DimensionScore { pub dimension: String, pub score: f32, pub evidence: String }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Genesis {
    pub dimensions: Vec<DimensionScore>,
    pub floor: f32,
    pub mean: f32,
    pub score: f32,
    pub threshold: f32,
    /// Aprovado quando o Genesis Score atinge o limiar do gênero (pipeline.md, etapa 8).
    pub approved: bool,
}

fn round1(value: f32) -> f32 { (value * 10.0).round() / 10.0 }

fn normalize(text: &str) -> String {
    text.to_lowercase().chars().filter(|c| c.is_alphanumeric()).map(|c| match c { 'á' | 'à' | 'â' | 'ã' => 'a', 'é' | 'ê' => 'e', 'í' => 'i', 'ó' | 'ô' | 'õ' => 'o', 'ú' => 'u', 'ç' => 'c', other => other }).collect()
}

/// Palavra-chave (já normalizada) que identifica cada dimensão no texto da IA.
const KEYWORDS: [&str; 7] = ["originalidade", "tema", "personagen", "prosa", "ritmo", "emocao", "criterio"];

/// Devolve `None` se a IA não entregou as 7 dimensões (nesse caso não inventamos um número).
pub fn compute(raw: &[RawScore], genre: Genre) -> Option<Genesis> {
    let mut dimensions: Vec<DimensionScore> = Vec::new();
    for (name, keyword) in DIMENSIONS.iter().zip(KEYWORDS) {
        let found = raw.iter().find(|r| normalize(&r.dimension).starts_with(keyword))?;
        if !found.score.is_finite() { return None; }
        dimensions.push(DimensionScore { dimension: name.to_string(), score: round1(found.score.clamp(0.0, 10.0)), evidence: found.evidence.trim().to_string() });
    }
    let floor = dimensions.iter().map(|d| d.score).fold(f32::MAX, f32::min);
    let mean = dimensions.iter().map(|d| d.score).sum::<f32>() / 7.0;
    let score = round1(floor * 0.4 + mean * 0.6);
    let threshold = genre.genesis_floor();
    Some(Genesis { dimensions, floor, mean: round1(mean), score, threshold, approved: score >= threshold })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(scores: [f32; 7]) -> Vec<RawScore> {
        DIMENSIONS.iter().zip(scores).map(|(d, s)| RawScore { dimension: d.to_string(), score: s, evidence: "ev".into() }).collect()
    }

    #[test]
    fn applies_the_skill_formula() {
        // floor 6.0, média 7.0 → 6*0.4 + 7*0.6 = 6.6
        let g = compute(&raw([8.0, 6.0, 7.0, 7.0, 7.0, 7.0, 7.0]), Genre::Literaria).unwrap();
        assert_eq!(g.floor, 6.0);
        assert_eq!(g.mean, 7.0);
        assert_eq!(g.score, 6.6);
        assert_eq!(g.threshold, 7.5);
        assert!(!g.approved);
    }

    #[test]
    fn approves_when_score_reaches_genre_threshold() {
        let g = compute(&raw([8.0; 7]), Genre::Thriller).unwrap();
        assert_eq!(g.score, 8.0);
        assert!(g.approved);
    }

    #[test]
    fn accepts_accent_and_case_variations_and_clamps() {
        let mut r = raw([7.0; 7]);
        r[4].dimension = "ritmo e coerencia".into();
        r[0].dimension = "ORIGINALIDADE".into();
        r[6].dimension = "Critério configurável".into();
        r[1].score = 15.0;
        let g = compute(&r, Genre::Comercial).unwrap();
        assert_eq!(g.dimensions[1].score, 10.0);
        assert_eq!(g.dimensions.len(), 7);
    }

    #[test]
    fn incomplete_scores_yield_no_number() {
        let mut r = raw([7.0; 7]);
        r.pop();
        assert!(compute(&r, Genre::Comercial).is_none());
        assert!(compute(&[], Genre::Comercial).is_none());
    }
}
