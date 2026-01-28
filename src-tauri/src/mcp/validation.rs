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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FrameHitbox, GuardType, MeterGain, Move, Pushback, Rect};

    fn make_valid_move() -> Move {
        Move {
            input: "5L".to_string(),
            name: "Standing Light".to_string(),
            startup: 7,
            active: 3,
            recovery: 8,
            damage: 30,
            hitstun: 17,
            blockstun: 11,
            hitstop: 6,
            guard: GuardType::Mid,
            hitboxes: vec![FrameHitbox {
                frames: (7, 9),
                r#box: Rect { x: 0, y: -40, w: 30, h: 16 },
            }],
            hurtboxes: vec![],
            pushback: Pushback { hit: 2, block: 2 },
            meter_gain: MeterGain { hit: 5, whiff: 2 },
            animation: "stand_light".to_string(),
        }
    }

    #[test]
    fn test_valid_move_passes() {
        let mv = make_valid_move();
        assert!(validate_move(&mv).is_ok());
    }

    #[test]
    fn test_zero_startup_fails() {
        let mut mv = make_valid_move();
        mv.startup = 0;
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "startup"));
    }

    #[test]
    fn test_zero_active_fails() {
        let mut mv = make_valid_move();
        mv.active = 0;
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "active"));
    }

    #[test]
    fn test_empty_input_fails() {
        let mut mv = make_valid_move();
        mv.input = "".to_string();
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "input"));
    }

    #[test]
    fn test_hitbox_frame_order_fails() {
        let mut mv = make_valid_move();
        mv.hitboxes[0].frames = (10, 5); // End before start
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field.contains("hitboxes")));
    }

    #[test]
    fn test_hitbox_exceeds_total_frames_fails() {
        let mut mv = make_valid_move();
        mv.hitboxes[0].frames = (7, 100); // Way beyond total
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("exceeds total frames")));
    }
}
