use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Specifies which moves a rule applies to. All specified fields must match (AND logic).
/// Within a single field, multiple values use OR logic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MatchSpec {
    /// Move type: normal, command_normal, special, super, movement, throw.
    #[serde(rename = "type")]
    pub r#type: Option<StringOrVec>,
    /// Button extracted from input (e.g., "236P" -> "P").
    pub button: Option<StringOrVec>,
    /// Guard type: high, mid, low, unblockable.
    pub guard: Option<StringOrVec>,
    /// Tags that must ALL be present on the move (AND logic).
    pub tags: Option<Vec<String>>,
    /// Input notation with glob pattern support (* matches any, ? matches one char).
    pub input: Option<StringOrVec>,
}

/// A value that can be either a single string or an array of strings.
/// Used for match criteria where OR logic is needed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum StringOrVec {
    /// A single value to match.
    One(String),
    /// Multiple values where any match satisfies the condition (OR logic).
    Many(Vec<String>),
}

pub(crate) fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();

    let mut dp = vec![vec![false; t.len() + 1]; p.len() + 1];
    dp[0][0] = true;

    for i in 1..=p.len() {
        if p[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        } else {
            break;
        }
    }

    for i in 1..=p.len() {
        for j in 1..=t.len() {
            dp[i][j] = match p[i - 1] {
                '*' => dp[i - 1][j] || dp[i][j - 1],
                '?' => dp[i - 1][j - 1],
                c => dp[i - 1][j - 1] && c == t[j - 1],
            };
        }
    }

    dp[p.len()][t.len()]
}

pub(crate) fn match_any<P>(patterns: &StringOrVec, value: &str, pred: P) -> bool
where
    P: Fn(&str, &str) -> bool,
{
    match patterns {
        StringOrVec::One(p) => pred(p, value),
        StringOrVec::Many(ps) => ps.iter().any(|p| pred(p, value)),
    }
}

pub(crate) fn button_from_input(input: &str) -> Option<&str> {
    let bytes = input.as_bytes();
    let mut i = bytes.len();
    while i > 0 && bytes[i - 1].is_ascii_alphabetic() {
        i -= 1;
    }
    if i == bytes.len() {
        None
    } else {
        Some(&input[i..])
    }
}

pub fn matches_move(spec: &MatchSpec, mv: &crate::schema::State) -> bool {
    if let Some(ty) = &spec.r#type {
        let mv_type = match &mv.move_type {
            Some(t) => t.as_str(),
            None => return false,
        };

        if !match_any(ty, mv_type, |p, v| p == v) {
            return false;
        }
    }

    if let Some(guard) = &spec.guard {
        let mv_guard = match mv.guard {
            crate::schema::GuardType::High => "high",
            crate::schema::GuardType::Mid => "mid",
            crate::schema::GuardType::Low => "low",
            crate::schema::GuardType::Unblockable => "unblockable",
        };

        if !match_any(guard, mv_guard, |p, v| p == v) {
            return false;
        }
    }

    if let Some(input) = &spec.input {
        if !match_any(input, &mv.input, glob_match) {
            return false;
        }
    }

    if let Some(button) = &spec.button {
        let mv_button = match button_from_input(&mv.input) {
            Some(b) => b,
            None => return false,
        };

        if !match_any(button, mv_button, |p, v| p == v) {
            return false;
        }
    }

    if let Some(tags) = &spec.tags {
        if !tags.iter().all(|t| mv.tags.iter().any(|tag| tag.as_str() == t)) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("236*", "236P"));
        assert!(!glob_match("236*", "623P"));

        assert!(glob_match("*P", "5P"));
        assert!(glob_match("*P", "623P"));
        assert!(!glob_match("*P", "5K"));

        assert!(glob_match("5?", "5L"));
        assert!(glob_match("5?", "5M"));
        assert!(!glob_match("5?", "5LL"));

        assert!(glob_match("236236*", "236236K"));
        assert!(!glob_match("236236*", "236K"));

        assert!(glob_match("[*]*", "[4]6P"));
    }

    #[test]
    fn test_matches_move_or_within_field_and_across_fields() {
        let mut mv = crate::schema::State::default();
        mv.input = "2L".to_string();
        mv.move_type = Some("command_normal".to_string());
        mv.guard = crate::schema::GuardType::Unblockable;

        // OR within a field
        let spec = MatchSpec {
            r#type: None,
            button: None,
            guard: None,
            tags: None,
            input: Some(StringOrVec::Many(vec!["5L".to_string(), "2L".to_string()])),
        };
        assert!(matches_move(&spec, &mv));

        // AND across fields
        let spec = MatchSpec {
            r#type: Some(StringOrVec::One("command_normal".to_string())),
            button: None,
            guard: Some(StringOrVec::One("unblockable".to_string())),
            tags: None,
            input: Some(StringOrVec::One("2L".to_string())),
        };
        assert!(matches_move(&spec, &mv));

        // AND across fields - guard mismatch
        let spec = MatchSpec {
            guard: Some(StringOrVec::One("mid".to_string())),
            ..spec
        };
        assert!(!matches_move(&spec, &mv));
    }

    #[test]
    fn test_matches_move_button_extraction() {
        let mut mv = crate::schema::State::default();
        mv.input = "j.H".to_string();

        let spec = MatchSpec {
            r#type: None,
            button: Some(StringOrVec::One("H".to_string())),
            guard: None,
            tags: None,
            input: None,
        };

        assert!(matches_move(&spec, &mv));

        mv.input = "632146PP".to_string();
        let spec = MatchSpec {
            r#type: None,
            button: Some(StringOrVec::One("PP".to_string())),
            guard: None,
            tags: None,
            input: None,
        };

        assert!(matches_move(&spec, &mv));
    }

    #[test]
    fn test_matches_move_tags_and() {
        let mut mv = crate::schema::State::default();
        mv.input = "5L".to_string();
        mv.tags = vec![
            crate::schema::Tag::new("starter").unwrap(),
            crate::schema::Tag::new("reversal").unwrap(),
        ];

        let spec = MatchSpec {
            r#type: None,
            button: None,
            guard: None,
            tags: Some(vec!["starter".to_string(), "reversal".to_string()]),
            input: None,
        };
        assert!(matches_move(&spec, &mv));

        let spec = MatchSpec {
            r#type: None,
            button: None,
            guard: None,
            tags: Some(vec!["starter".to_string(), "missing".to_string()]),
            input: None,
        };
        assert!(!matches_move(&spec, &mv));
    }
}
