use crate::schema::Move;

#[derive(Debug)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

pub fn validate_move(mv: &Move) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // Frame data sanity
    if mv.startup == 0 {
        errors.push(ValidationError {
            field: "startup".to_string(),
            message: "startup must be at least 1 frame".to_string(),
        });
    }

    if mv.active == 0 {
        errors.push(ValidationError {
            field: "active".to_string(),
            message: "active must be at least 1 frame".to_string(),
        });
    }

    // Input validation
    if mv.input.is_empty() {
        errors.push(ValidationError {
            field: "input".to_string(),
            message: "input cannot be empty".to_string(),
        });
    }

    // Hitbox frame range validation
    let total_frames = mv.startup + mv.active + mv.recovery;
    for (i, hitbox) in mv.hitboxes.iter().enumerate() {
        if hitbox.frames.0 > hitbox.frames.1 {
            errors.push(ValidationError {
                field: format!("hitboxes[{}].frames", i),
                message: "start frame cannot be after end frame".to_string(),
            });
        }
        if hitbox.frames.1 > total_frames {
            errors.push(ValidationError {
                field: format!("hitboxes[{}].frames", i),
                message: format!("end frame {} exceeds total frames {}", hitbox.frames.1, total_frames),
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
