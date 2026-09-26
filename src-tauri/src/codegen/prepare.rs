use crate::rules::RulesFile;
use crate::schema::{CancelTable, Character, CharacterData, State};

/// The same validation/defaults pipeline for native export and browser authoring.
pub fn prepare_character(
    character: Character,
    base_moves: Vec<State>,
    cancel_table: CancelTable,
    project_rules: Option<&RulesFile>,
    character_rules: Option<&RulesFile>,
) -> Result<CharacterData, String> {
    let mut error_messages = Vec::new();

    let registry = crate::rules::merged_registry(project_rules, character_rules);
    let char_issues =
        crate::rules::validate_character_resources_with_registry(&character, &registry);
    error_messages.extend(
        char_issues
            .into_iter()
            .filter(|i| i.severity == crate::rules::Severity::Error)
            .map(|i| format!("character {}: {}", i.field, i.message)),
    );

    let mut resolved_moves = Vec::with_capacity(base_moves.len());
    for mv in base_moves {
        let issues = crate::rules::validate_move_with_rules(project_rules, character_rules, &mv)
            .map_err(|e| format!("Failed to validate move '{}': {}", mv.input, e))?;

        error_messages.extend(
            issues
                .into_iter()
                .filter(|i| i.severity == crate::rules::Severity::Error)
                .map(|i| format!("{} {}: {}", mv.input, i.field, i.message)),
        );

        let resolved = crate::rules::apply_rules_to_move(project_rules, character_rules, &mv)
            .map_err(|e| format!("Failed to apply rules to move '{}': {}", mv.input, e))?;
        resolved_moves.push(resolved);
    }

    if !error_messages.is_empty() {
        return Err(error_messages.join("; "));
    }

    Ok(CharacterData {
        character,
        moves: resolved_moves,
        cancel_table,
    })
}
