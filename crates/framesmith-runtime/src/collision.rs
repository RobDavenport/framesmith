// TODO: Implement collision logic (Task 7, 8)

/// Result of a hit check between two entities.
#[derive(Clone, Copy, Debug, Default)]
pub struct HitResult {
    /// True if a hit was detected.
    pub hit: bool,
}

/// Check for hits between entities.
pub fn check_hits() -> HitResult {
    todo!("Implement in Task 8")
}

/// Check if two axis-aligned shapes overlap.
pub fn shapes_overlap() -> bool {
    todo!("Implement in Task 7")
}
