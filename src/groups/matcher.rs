use regex::Regex;
use serde::{Deserialize, Serialize};

/// Result of matching a filename against a group rule.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchResult {
    pub kind: GroupKind,
    pub canonical_name: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GroupKind {
    Series,
    Movie,
    Collection,
}

impl GroupKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            GroupKind::Series => "series",
            GroupKind::Movie => "movie",
            GroupKind::Collection => "collection",
        }
    }
}

/// Remove file extension, lowercase, and normalize separators.
fn normalize_separators(name: &str) -> String {
    let name = name.rsplit_once('.').map(|(base, _)| base).unwrap_or(name);
    let name = name.to_lowercase();
    let name = name.replace(['.', '_', '-'], " ");
    Regex::new(r"\s+").unwrap().replace_all(&name, " ").trim().to_string()
}

/// Clean a title by removing metadata tokens (quality, release tags, brackets).
fn clean_title(name: &str) -> String {
    let name = name.to_lowercase();
    let name = name.replace(['.', '_', '-'], " ");
    let name = Regex::new(r"\b(1080p|720p|2160p|4k|8k|hdr|dv|blu-?ray|remux|web-?dl|webrip|hdtv|x264|x265|hevc|avc|aac|dts|hdma|truehd|atmos)\b").unwrap().replace_all(&name, " ");
    let name = Regex::new(r"[\(\[\{].*?[\)\]\}]").unwrap().replace_all(&name, " ");
    Regex::new(r"\s+").unwrap().replace_all(&name, " ").trim().to_string()
}

/// Detect if a filename represents a TV episode.
fn detect_series(name: &str) -> Option<MatchResult> {
    // Find the episode marker (S01E02, 1x02, etc.) and take everything before it.
    let re = Regex::new(r"(?i)[.\s-]+[Ss]?(\d{1,2})[Eex](\d{1,2})").unwrap();
    let m = re.find(name)?;
    let raw = &name[..m.start()];
    let cleaned = clean_title(raw);
    if cleaned.is_empty() || cleaned.split_whitespace().count() < 1 {
        return None;
    }
    let canonical = format!("series:{}", cleaned.replace(' ', "."));
    Some(MatchResult {
        kind: GroupKind::Series,
        canonical_name: canonical,
        display_name: cleaned,
    })
}

/// Detect if a filename represents a movie (possibly multiple versions).
fn detect_movie(name: &str) -> Option<MatchResult> {
    // Strip extension and clean separators first.
    let base = normalize_separators(name);

    // Look for a 4-digit year (1900-2099) as a strong movie signal.
    // We search in the raw normalized name so that years inside parentheses
    // are still detected before brackets are stripped.
    let year_re = Regex::new(r"\b(19\d{2}|20\d{2})\b").unwrap();
    if !year_re.is_match(&base) {
        return None;
    }

    let cleaned = clean_title(&base);
    if cleaned.is_empty() {
        return None;
    }

    // Remove year and version/remaster markers to get canonical title.
    let canonical_base = year_re.replace_all(&cleaned, " ").trim().to_string();
    let canonical_base = Regex::new(r"\b(director|directors|extended|uncut|remastered|theatrical|ultimate|final|cut|edition|version|part|vol|volume)\b").unwrap()
        .replace_all(&canonical_base, " ")
        .trim()
        .to_string();
    let canonical_base = Regex::new(r"\s+").unwrap().replace_all(&canonical_base, " ").to_string();
    if canonical_base.is_empty() || canonical_base.split_whitespace().count() < 1 {
        return None;
    }
    let canonical = format!("movie:{}", canonical_base.replace(' ', "."));
    Some(MatchResult {
        kind: GroupKind::Movie,
        canonical_name: canonical,
        display_name: canonical_base,
    })
}

/// Fallback: group by shared prefix of the first N words.
fn detect_collection(name: &str) -> Option<MatchResult> {
    let cleaned = clean_title(&normalize_separators(name));
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    if words.len() >= 2 {
        let prefix_words = &words[..words.len().min(3)];
        let prefix = prefix_words.join(" ");
        if prefix.len() >= 3 {
            let canonical = format!("collection:{}", prefix.replace(' ', "."));
            return Some(MatchResult {
                kind: GroupKind::Collection,
                canonical_name: canonical,
                display_name: prefix,
            });
        }
    }
    None
}

/// Match a filename and return the best group classification.
pub fn match_name(name: &str) -> Option<MatchResult> {
    detect_series(name)
        .or_else(|| detect_movie(name))
        .or_else(|| detect_collection(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_series_patterns() {
        let cases = [
            ("Show.Name.S01E02.mp4", GroupKind::Series, "show name"),
            ("Show Name - S1E2.mkv", GroupKind::Series, "show name"),
            ("My.Show.1x02.avi", GroupKind::Series, "my show"),
        ];
        for (input, expected_kind, expected_name) in cases {
            let result = match_name(input).expect("should match series");

            assert_eq!(result.kind, expected_kind);
            assert_eq!(result.display_name, expected_name);
            assert!(result.canonical_name.starts_with("series:"));
        }
    }

    #[test]
    fn test_movie_patterns() {
        let cases = [
            ("Movie.Name.2024.1080p.BluRay.mp4", GroupKind::Movie, "movie name"),
            ("Movie Name (2024) Directors Cut.mkv", GroupKind::Movie, "movie name"),
        ];
        for (input, expected_kind, expected_name) in cases {
            let result = match_name(input).expect("should match movie");

            assert_eq!(result.kind, expected_kind);
            assert_eq!(result.display_name, expected_name);
            assert!(result.canonical_name.starts_with("movie:"));
        }
    }

    #[test]
    fn test_collection_fallback() {
        let result = match_name("Some Random File.txt").expect("should match collection");
        assert_eq!(result.kind, GroupKind::Collection);
        assert_eq!(result.display_name, "some random file");
    }
}
