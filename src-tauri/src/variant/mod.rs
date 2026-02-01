//! Variant resolution for state inheritance.
//!
//! Variants allow states to inherit from base states with targeted overrides.
//! Filename convention: `{base}~{variant}.json` (e.g., `5H~level1.json`)

/// Parse a state name into (base, variant) components.
///
/// Splits on the **last** tilde. If the portion after the last tilde is empty,
/// treats the whole name as a base state (e.g., `5S~` is a hold input, not a variant).
pub fn parse_variant_name(name: &str) -> (&str, Option<&str>) {
    match name.rfind('~') {
        Some(pos) => {
            let variant_part = &name[pos + 1..];
            if variant_part.is_empty() {
                (name, None)
            } else {
                (&name[..pos], Some(variant_part))
            }
        }
        None => (name, None),
    }
}

/// Check if a filename represents a variant (has non-empty variant portion).
pub fn is_variant_filename(name: &str) -> bool {
    parse_variant_name(name).1.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_base_state_no_tilde() {
        let (base, variant) = parse_variant_name("5H");
        assert_eq!(base, "5H");
        assert_eq!(variant, None);
    }

    #[test]
    fn parse_simple_variant() {
        let (base, variant) = parse_variant_name("5H~level1");
        assert_eq!(base, "5H");
        assert_eq!(variant, Some("level1"));
    }

    #[test]
    fn parse_hold_notation_as_base() {
        let (base, variant) = parse_variant_name("5S~");
        assert_eq!(base, "5S~");
        assert_eq!(variant, None);
    }

    #[test]
    fn parse_hold_variant() {
        let (base, variant) = parse_variant_name("5S~~installed");
        assert_eq!(base, "5S~");
        assert_eq!(variant, Some("installed"));
    }

    #[test]
    fn parse_rekka_notation() {
        let (base, variant) = parse_variant_name("236K~K");
        assert_eq!(base, "236K");
        assert_eq!(variant, Some("K"));
    }

    #[test]
    fn is_variant_checks_correctly() {
        assert!(!is_variant_filename("5H"));
        assert!(is_variant_filename("5H~level1"));
        assert!(!is_variant_filename("5S~"));
        assert!(is_variant_filename("5S~~installed"));
    }
}
