//! Gêneros e limiares da skill editor-profissional-de-livros
//! (`references/analise.md` e `references/scores.md`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Genre {
    Literaria,
    Memoir,
    Comercial,
    Thriller,
    Romance,
    Sff,
    NaoFiccao,
}

impl Default for Genre {
    fn default() -> Self { Genre::Comercial }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenreInfo {
    pub id: Genre,
    pub label: &'static str,
}

pub const GENRES: &[GenreInfo] = &[
    GenreInfo { id: Genre::Literaria, label: "Ficção literária" },
    GenreInfo { id: Genre::Memoir, label: "Memoir" },
    GenreInfo { id: Genre::Comercial, label: "Ficção comercial" },
    GenreInfo { id: Genre::Thriller, label: "Thriller / Suspense" },
    GenreInfo { id: Genre::Romance, label: "Romance" },
    GenreInfo { id: Genre::Sff, label: "Ficção científica / Fantasia" },
    GenreInfo { id: Genre::NaoFiccao, label: "Não-ficção prescritiva" },
];

impl Genre {
    pub fn label(self) -> &'static str {
        GENRES.iter().find(|info| info.id == self).map(|info| info.label).unwrap_or("Ficção comercial")
    }

    /// Máximo de padrões anti-IA aceitável (analise.md). A skill não lista
    /// thriller, romance e SFF; seguem a faixa de ficção comercial (premissa).
    pub fn anti_ia_max(self) -> u32 {
        match self {
            Genre::Literaria => 3,
            Genre::Memoir => 4,
            Genre::NaoFiccao => 12,
            Genre::Comercial | Genre::Thriller | Genre::Romance | Genre::Sff => 8,
        }
    }

    /// Faixa-alvo de diálogo em % (analise.md). `None` quando a skill não define.
    pub fn dialogue_range(self) -> Option<(u32, u32)> {
        match self {
            Genre::Literaria => Some((15, 35)),
            Genre::Memoir => Some((10, 30)),
            Genre::Thriller => Some((30, 50)),
            Genre::NaoFiccao => Some((0, 15)),
            Genre::Romance => Some((30, 45)),
            Genre::Comercial | Genre::Sff => None,
        }
    }

    /// Floor mínimo do Genesis Score (scores.md). Romance e SFF não são listados:
    /// usamos o de ficção comercial (premissa).
    pub fn genesis_floor(self) -> f32 {
        match self {
            Genre::Literaria | Genre::Memoir => 7.5,
            Genre::NaoFiccao => 6.5,
            Genre::Comercial | Genre::Thriller | Genre::Romance | Genre::Sff => 7.0,
        }
    }

    /// Meta de símiles com extensão explanatória (padrão #11) por texto de ~3 mil palavras.
    pub fn simile_extension_max(self) -> u32 {
        match self {
            Genre::Literaria => 3,
            Genre::Memoir => 4,
            Genre::NaoFiccao => 8,
            _ => 6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_follow_the_skill_tables() {
        assert_eq!(Genre::Literaria.anti_ia_max(), 3);
        assert_eq!(Genre::NaoFiccao.anti_ia_max(), 12);
        assert_eq!(Genre::Thriller.dialogue_range(), Some((30, 50)));
        assert_eq!(Genre::Comercial.dialogue_range(), None);
        assert_eq!(Genre::Memoir.genesis_floor(), 7.5);
        assert_eq!(Genre::NaoFiccao.genesis_floor(), 6.5);
        assert_eq!(serde_json::to_string(&Genre::NaoFiccao).unwrap(), "\"nao-ficcao\"");
        assert_eq!(serde_json::from_str::<Genre>("\"sff\"").unwrap(), Genre::Sff);
    }
}
