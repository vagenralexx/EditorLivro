//! Funções pequenas e puras usadas pelos demais módulos.

use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Codifica um caminho relativo para uso em `href`/`src` (preserva `/`).
pub fn percent_encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub fn percent_decode(text: &str) -> String {
    fn hex(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex(bytes[index + 1]), hex(bytes[index + 2])) {
                out.push(high * 16 + low);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Ordenação "natural": capitulo2 < capitulo10, sem diferenciar maiúsculas.
pub fn natural_cmp(left: &str, right: &str) -> Ordering {
    let (mut a, mut b) = (left.chars().peekable(), right.chars().peekable());
    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(&c) = a.peek() { if c.is_ascii_digit() { na.push(c); a.next(); } else { break; } }
                let mut nb = String::new();
                while let Some(&c) = b.peek() { if c.is_ascii_digit() { nb.push(c); b.next(); } else { break; } }
                let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let order = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if order != Ordering::Equal { return order; }
            }
            (Some(x), Some(y)) => {
                let order = x.to_lowercase().cmp(y.to_lowercase());
                if order != Ordering::Equal { return order; }
                a.next();
                b.next();
            }
        }
    }
}

pub fn extension(path: &str) -> String {
    path.rsplit('.').next().filter(|ext| !ext.contains('/')).unwrap_or("").to_ascii_lowercase()
}

pub fn file_stem(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rfind('.') {
        Some(dot) if dot > 0 => name[..dot].to_string(),
        _ => name.to_string(),
    }
}

pub fn dir_of(path: &str) -> &str {
    match path.rfind('/') {
        Some(slash) => &path[..slash],
        None => "",
    }
}

/// Media types aceitos como recursos centrais do EPUB 3.
pub fn media_type_for(path: &str) -> Option<&'static str> {
    Some(match extension(path).as_str() {
        "xhtml" | "html" | "htm" => "application/xhtml+xml",
        "css" => "text/css",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "otf" => "font/otf",
        "ttf" => "font/ttf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ncx" => "application/x-dtbncx+xml",
        "js" => "text/javascript",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        _ => return None,
    })
}

/// Resolve `href` (com `./` e `../`) relativo ao arquivo `from`. Devolve `None`
/// se o caminho escapar da raiz do pacote.
pub fn resolve_href(from_file: &str, href: &str) -> Option<String> {
    let mut parts: Vec<&str> = if href.starts_with('/') { Vec::new() } else { dir_of(from_file).split('/').filter(|p| !p.is_empty()).collect() };
    for segment in href.split('/') {
        match segment {
            "" | "." => {}
            ".." => { parts.pop()?; }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

/// Caminho relativo de `to` visto de dentro do arquivo `from` (ambos relativos à mesma raiz).
pub fn relative_href(from_file: &str, to: &str) -> String {
    let from_dir: Vec<&str> = dir_of(from_file).split('/').filter(|p| !p.is_empty()).collect();
    let target: Vec<&str> = to.split('/').filter(|p| !p.is_empty()).collect();
    let common = from_dir.iter().zip(target.iter()).take_while(|(a, b)| a == b).count().min(target.len().saturating_sub(1));
    let mut parts: Vec<String> = vec!["..".to_string(); from_dir.len() - common];
    parts.extend(target[common..].iter().map(|s| s.to_string()));
    parts.join("/")
}

/// Data/hora UTC no formato exigido por `dcterms:modified` (CCYY-MM-DDThh:mm:ssZ).
pub fn utc_timestamp() -> String {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, rem) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rem / 3600, (rem % 3600) / 60, rem % 60)
}

// Algoritmo de Howard Hinnant (domínio público).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Substitui entidades HTML nomeadas que não existem em XML puro (um XHTML com
/// `&nbsp;` é rejeitado por leitores e pelo epubcheck). Devolve o texto e o
/// número de trocas.
pub fn fix_html_entities(text: &str) -> (String, usize) {
    const ENTITIES: &[(&str, &str)] = &[
        ("&nbsp;", "&#160;"), ("&ensp;", "&#8194;"), ("&emsp;", "&#8195;"), ("&thinsp;", "&#8201;"),
        ("&shy;", "&#173;"), ("&hellip;", "&#8230;"), ("&mdash;", "&#8212;"), ("&ndash;", "&#8211;"),
        ("&lsquo;", "&#8216;"), ("&rsquo;", "&#8217;"), ("&ldquo;", "&#8220;"), ("&rdquo;", "&#8221;"),
        ("&laquo;", "&#171;"), ("&raquo;", "&#187;"), ("&copy;", "&#169;"), ("&middot;", "&#183;"),
    ];
    let mut out = text.to_string();
    let mut count = 0;
    for (named, numeric) in ENTITIES {
        let found = out.matches(named).count();
        if found > 0 {
            out = out.replace(named, numeric);
            count += found;
        }
    }
    (out, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_order_sorts_numbers_by_value() {
        let mut names = vec!["cap10.xhtml", "cap2.xhtml", "Cap1.xhtml", "cap02b.xhtml"];
        names.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(names, vec!["Cap1.xhtml", "cap2.xhtml", "cap02b.xhtml", "cap10.xhtml"]);
    }

    #[test]
    fn percent_round_trip_handles_accents_and_spaces() {
        let encoded = percent_encode_path("text/Capítulo 1.xhtml");
        assert_eq!(encoded, "text/Cap%C3%ADtulo%201.xhtml");
        assert_eq!(percent_decode(&encoded), "text/Capítulo 1.xhtml");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("a%2"), "a%2");
    }

    #[test]
    fn href_resolution_and_relative_paths() {
        assert_eq!(resolve_href("OEBPS/text/a.xhtml", "../css/s.css").as_deref(), Some("OEBPS/css/s.css"));
        assert_eq!(resolve_href("a.xhtml", "../x"), None);
        assert_eq!(relative_href("text/a.xhtml", "css/s.css"), "../css/s.css");
        assert_eq!(relative_href("a.xhtml", "css/s.css"), "css/s.css");
        assert_eq!(relative_href("text/a.xhtml", "text/b.xhtml"), "b.xhtml");
    }

    #[test]
    fn timestamp_has_epub_format() {
        let stamp = utc_timestamp();
        assert_eq!(stamp.len(), 20);
        assert!(stamp.ends_with('Z') && stamp.as_bytes()[10] == b'T');
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
    }

    #[test]
    fn html_entities_are_made_xml_safe() {
        let (fixed, count) = fix_html_entities("a&nbsp;b &mdash; c &amp; d");
        assert_eq!(fixed, "a&#160;b &#8212; c &amp; d");
        assert_eq!(count, 2);
    }
}
