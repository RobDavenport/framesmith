use crate::schema::Character;

/// Export character data as a single minified JSON blob
pub fn export_json_blob(_character: &Character) -> Result<String, String> {
    // TODO: Implement JSON blob export
    Ok(String::new())
}
