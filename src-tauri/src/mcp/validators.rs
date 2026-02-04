use crate::schema::{
    Cost, FrameHurtbox, Hit, HitboxShape, Movement, Precondition, StatusEffect, SuperFreeze,
};

use super::validation::ValidationError;

pub fn validate_hits(hits: &[Hit], errors: &mut Vec<ValidationError>) {
    for (i, hit) in hits.iter().enumerate() {
        let prefix = format!("hits[{}]", i);

        // Frame order validation
        if hit.frames.0 > hit.frames.1 {
            errors.push(ValidationError {
                field: format!("{}.frames", prefix),
                message: "start frame cannot be after end frame".to_string(),
            });
        }

        // Validate hitbox shapes within each hit
        for (j, hitbox) in hit.hitboxes.iter().enumerate() {
            validate_hitbox_shape(hitbox, &format!("{}.hitboxes[{}]", prefix, j), errors);
        }
    }
}

pub fn validate_hitbox_shape(
    shape: &HitboxShape,
    field_prefix: &str,
    errors: &mut Vec<ValidationError>,
) {
    match shape {
        HitboxShape::Aabb { w, h, .. } | HitboxShape::Rect { w, h, .. } => {
            if *w == 0 {
                errors.push(ValidationError {
                    field: format!("{}.w", field_prefix),
                    message: "width must be greater than 0".to_string(),
                });
            }
            if *h == 0 {
                errors.push(ValidationError {
                    field: format!("{}.h", field_prefix),
                    message: "height must be greater than 0".to_string(),
                });
            }
        }
        HitboxShape::Circle { r, .. } => {
            if *r == 0 {
                errors.push(ValidationError {
                    field: format!("{}.r", field_prefix),
                    message: "radius must be greater than 0".to_string(),
                });
            }
        }
        HitboxShape::Capsule { r, .. } => {
            if *r == 0 {
                errors.push(ValidationError {
                    field: format!("{}.r", field_prefix),
                    message: "radius must be greater than 0".to_string(),
                });
            }
        }
    }
}

pub fn validate_preconditions(preconditions: &[Precondition], errors: &mut Vec<ValidationError>) {
    for (i, precondition) in preconditions.iter().enumerate() {
        let prefix = format!("preconditions[{}]", i);

        match precondition {
            Precondition::Meter { min, max } => {
                if let (Some(min_val), Some(max_val)) = (min, max) {
                    if min_val > max_val {
                        errors.push(ValidationError {
                            field: prefix,
                            message: "meter min cannot be greater than max".to_string(),
                        });
                    }
                }
            }
            Precondition::Charge { min_frames, .. } => {
                if *min_frames == 0 {
                    errors.push(ValidationError {
                        field: format!("{}.min_frames", prefix),
                        message: "charge min_frames must be greater than 0".to_string(),
                    });
                }
            }
            Precondition::Health {
                min_percent,
                max_percent,
            } => {
                if let Some(min_val) = min_percent {
                    if *min_val > 100 {
                        errors.push(ValidationError {
                            field: format!("{}.min_percent", prefix),
                            message: "health min_percent cannot exceed 100".to_string(),
                        });
                    }
                }
                if let Some(max_val) = max_percent {
                    if *max_val > 100 {
                        errors.push(ValidationError {
                            field: format!("{}.max_percent", prefix),
                            message: "health max_percent cannot exceed 100".to_string(),
                        });
                    }
                }
                if let (Some(min_val), Some(max_val)) = (min_percent, max_percent) {
                    if min_val > max_val {
                        errors.push(ValidationError {
                            field: prefix,
                            message: "health min_percent cannot be greater than max_percent"
                                .to_string(),
                        });
                    }
                }
            }
            Precondition::EntityCount { min, max, .. } => {
                if let (Some(min_val), Some(max_val)) = (min, max) {
                    if min_val > max_val {
                        errors.push(ValidationError {
                            field: prefix,
                            message: "entity_count min cannot be greater than max".to_string(),
                        });
                    }
                }
            }
            Precondition::Resource { min, max, .. } => {
                if let (Some(min_val), Some(max_val)) = (min, max) {
                    if min_val > max_val {
                        errors.push(ValidationError {
                            field: prefix,
                            message: "resource min cannot be greater than max".to_string(),
                        });
                    }
                }
            }
            Precondition::ComboCount { min, max } => {
                if let (Some(min_val), Some(max_val)) = (min, max) {
                    if min_val > max_val {
                        errors.push(ValidationError {
                            field: prefix,
                            message: "combo_count min cannot be greater than max".to_string(),
                        });
                    }
                }
            }
            Precondition::Distance { min, max } => {
                if let (Some(min_val), Some(max_val)) = (min, max) {
                    if min_val > max_val {
                        errors.push(ValidationError {
                            field: prefix,
                            message: "distance min cannot be greater than max".to_string(),
                        });
                    }
                }
            }
            // These preconditions don't need additional validation
            Precondition::State { .. }
            | Precondition::Grounded
            | Precondition::Airborne
            | Precondition::OpponentState { .. } => {}
        }
    }
}

pub fn validate_costs(costs: &[Cost], errors: &mut Vec<ValidationError>) {
    for (i, cost) in costs.iter().enumerate() {
        let prefix = format!("costs[{}]", i);

        let amount = match cost {
            Cost::Meter { amount } => *amount,
            Cost::Health { amount } => *amount,
            Cost::Resource { amount, .. } => *amount,
        };

        if amount == 0 {
            errors.push(ValidationError {
                field: format!("{}.amount", prefix),
                message: "cost amount must be greater than 0".to_string(),
            });
        }
    }
}

pub fn validate_movement(movement: &Movement, errors: &mut Vec<ValidationError>) {
    // Movement can be distance-based or velocity-based
    // Distance-based: distance field is set
    // Velocity-based: velocity field is set

    let is_distance_based = movement.distance.is_some();
    let is_velocity_based = movement.velocity.is_some();

    if !is_distance_based && !is_velocity_based {
        errors.push(ValidationError {
            field: "movement".to_string(),
            message: "movement must have either distance or velocity defined".to_string(),
        });
        return;
    }

    if let Some(distance) = movement.distance {
        if distance == 0 {
            errors.push(ValidationError {
                field: "movement.distance".to_string(),
                message: "movement distance must be greater than 0".to_string(),
            });
        }
    }
}

pub fn validate_super_freeze(super_freeze: &SuperFreeze, errors: &mut Vec<ValidationError>) {
    if super_freeze.frames == 0 {
        errors.push(ValidationError {
            field: "super_freeze.frames".to_string(),
            message: "super_freeze frames must be greater than 0".to_string(),
        });
    }

    if let Some(zoom) = super_freeze.zoom {
        if zoom <= 0.0 {
            errors.push(ValidationError {
                field: "super_freeze.zoom".to_string(),
                message: "super_freeze zoom must be greater than 0".to_string(),
            });
        }
    }

    if let Some(darken) = super_freeze.darken {
        if !(0.0..=1.0).contains(&darken) {
            errors.push(ValidationError {
                field: "super_freeze.darken".to_string(),
                message: "super_freeze darken must be between 0.0 and 1.0".to_string(),
            });
        }
    }
}

pub fn validate_status_effects(
    effects: &[StatusEffect],
    prefix: &str,
    errors: &mut Vec<ValidationError>,
) {
    for (i, effect) in effects.iter().enumerate() {
        let field_prefix = format!("{}[{}]", prefix, i);

        match effect {
            StatusEffect::Poison {
                damage_per_frame,
                duration,
            }
            | StatusEffect::Burn {
                damage_per_frame,
                duration,
            } => {
                if *damage_per_frame == 0 {
                    errors.push(ValidationError {
                        field: format!("{}.damage_per_frame", field_prefix),
                        message: "damage_per_frame must be greater than 0".to_string(),
                    });
                }
                if *duration == 0 {
                    errors.push(ValidationError {
                        field: format!("{}.duration", field_prefix),
                        message: "duration must be greater than 0".to_string(),
                    });
                }
            }
            StatusEffect::Stun { duration } | StatusEffect::Weaken { duration, .. } => {
                if *duration == 0 {
                    errors.push(ValidationError {
                        field: format!("{}.duration", field_prefix),
                        message: "duration must be greater than 0".to_string(),
                    });
                }
            }
            StatusEffect::Slow {
                multiplier,
                duration,
            } => {
                if *duration == 0 {
                    errors.push(ValidationError {
                        field: format!("{}.duration", field_prefix),
                        message: "duration must be greater than 0".to_string(),
                    });
                }
                if *multiplier < 0.0 || *multiplier > 1.0 {
                    errors.push(ValidationError {
                        field: format!("{}.multiplier", field_prefix),
                        message: "slow multiplier must be between 0.0 and 1.0".to_string(),
                    });
                }
            }
            StatusEffect::Seal { duration, .. } => {
                if *duration == 0 {
                    errors.push(ValidationError {
                        field: format!("{}.duration", field_prefix),
                        message: "duration must be greater than 0".to_string(),
                    });
                }
            }
        }
    }
}

pub fn validate_frame_hurtboxes(hurtboxes: &[FrameHurtbox], errors: &mut Vec<ValidationError>) {
    for (i, hurtbox) in hurtboxes.iter().enumerate() {
        let prefix = format!("advanced_hurtboxes[{}]", i);

        // Frame order validation
        if hurtbox.frames.0 > hurtbox.frames.1 {
            errors.push(ValidationError {
                field: format!("{}.frames", prefix),
                message: "start frame cannot be after end frame".to_string(),
            });
        }

        // Validate each box shape
        for (j, shape) in hurtbox.boxes.iter().enumerate() {
            validate_hitbox_shape(shape, &format!("{}.boxes[{}]", prefix, j), errors);
        }
    }
}
