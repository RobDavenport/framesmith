use crate::schema::{
    Cost, FrameHurtbox, Hit, HitboxShape, Movement, Precondition, State, StatusEffect, SuperFreeze,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

pub fn validate_move(mv: &State) -> Result<(), Vec<ValidationError>> {
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

    // Legacy frame range validation for hitboxes/hurtboxes.
    // Use explicit mv.total when present (v2 schema), otherwise derive total from S/A/R.
    let effective_total_frames: u16 = mv.total.map(u16::from).unwrap_or_else(|| {
        u16::from(mv.startup)
            .saturating_add(u16::from(mv.active))
            .saturating_add(u16::from(mv.recovery))
    });

    for (i, hitbox) in mv.hitboxes.iter().enumerate() {
        if hitbox.frames.0 > hitbox.frames.1 {
            errors.push(ValidationError {
                field: format!("hitboxes[{}].frames", i),
                message: "start frame cannot be after end frame".to_string(),
            });
        }
        if u16::from(hitbox.frames.1) > effective_total_frames {
            errors.push(ValidationError {
                field: format!("hitboxes[{}].frames", i),
                message: format!(
                    "end frame {} exceeds total frames {}",
                    hitbox.frames.1, effective_total_frames
                ),
            });
        }
    }

    for (i, hurtbox) in mv.hurtboxes.iter().enumerate() {
        if hurtbox.frames.0 > hurtbox.frames.1 {
            errors.push(ValidationError {
                field: format!("hurtboxes[{}].frames", i),
                message: "start frame cannot be after end frame".to_string(),
            });
        }
        if u16::from(hurtbox.frames.1) > effective_total_frames {
            errors.push(ValidationError {
                field: format!("hurtboxes[{}].frames", i),
                message: format!(
                    "end frame {} exceeds total frames {}",
                    hurtbox.frames.1, effective_total_frames
                ),
            });
        }
    }

    // Validate hits array (v2 schema)
    if let Some(ref hits) = mv.hits {
        validate_hits(hits, &mut errors);
    }

    // Validate preconditions (v2 schema)
    if let Some(ref preconditions) = mv.preconditions {
        validate_preconditions(preconditions, &mut errors);
    }

    // Validate costs (v2 schema)
    if let Some(ref costs) = mv.costs {
        validate_costs(costs, &mut errors);
    }

    // Validate movement (v2 schema)
    if let Some(ref movement) = mv.movement {
        validate_movement(movement, &mut errors);
    }

    // Validate super_freeze (v2 schema)
    if let Some(ref super_freeze) = mv.super_freeze {
        validate_super_freeze(super_freeze, &mut errors);
    }

    // Validate on_hit status effects (v2 schema)
    if let Some(ref on_hit) = mv.on_hit {
        if let Some(ref status) = on_hit.status {
            validate_status_effects(status, "on_hit.status", &mut errors);
        }
    }

    // Validate advanced_hurtboxes (v2 schema)
    if let Some(ref hurtboxes) = mv.advanced_hurtboxes {
        validate_frame_hurtboxes(hurtboxes, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_hits(hits: &[Hit], errors: &mut Vec<ValidationError>) {
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

fn validate_hitbox_shape(
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

fn validate_preconditions(preconditions: &[Precondition], errors: &mut Vec<ValidationError>) {
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

fn validate_costs(costs: &[Cost], errors: &mut Vec<ValidationError>) {
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

fn validate_movement(movement: &Movement, errors: &mut Vec<ValidationError>) {
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

fn validate_super_freeze(super_freeze: &SuperFreeze, errors: &mut Vec<ValidationError>) {
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

fn validate_status_effects(
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

fn validate_frame_hurtboxes(hurtboxes: &[FrameHurtbox], errors: &mut Vec<ValidationError>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        Cost, FrameHitbox, FrameHurtbox, GuardType, Hit, HitboxShape, MeterGain, Movement, OnHit,
        Precondition, Pushback, Rect, State, StatusEffect, SuperFreeze,
    };

    fn make_valid_move() -> State {
        State {
            input: "5L".to_string(),
            name: "Standing Light".to_string(),
            tags: vec![],
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
                r#box: Rect {
                    x: 0,
                    y: -40,
                    w: 30,
                    h: 16,
                },
            }],
            hurtboxes: vec![],
            pushback: Pushback { hit: 2, block: 2 },
            meter_gain: MeterGain { hit: 5, whiff: 2 },
            animation: "stand_light".to_string(),
            // v2 optional fields
            hits: None,
            preconditions: None,
            costs: None,
            movement: None,
            super_freeze: None,
            on_hit: None,
            advanced_hurtboxes: None,
            move_type: None,
            trigger: None,
            parent: None,
            total: None,
            on_use: None,
            on_block: None,
            notifies: vec![],
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
        assert!(errors
            .iter()
            .any(|e| e.message.contains("exceeds total frames")));
    }

    #[test]
    fn test_hitbox_ending_at_explicit_total_passes() {
        let mut mv = make_valid_move();
        mv.total = Some(25);
        mv.hitboxes[0].frames = (7, 25);
        assert!(validate_move(&mv).is_ok());
    }

    #[test]
    fn test_hurtbox_invalid_frame_order_fails() {
        let mut mv = make_valid_move();
        mv.hurtboxes = vec![FrameHitbox {
            frames: (10, 5),
            r#box: Rect {
                x: 0,
                y: -40,
                w: 30,
                h: 16,
            },
        }];

        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "hurtboxes[0].frames"));
    }

    // ========== v2 Schema Tests ==========

    #[test]
    fn test_hit_frame_order_fails() {
        let mut mv = make_valid_move();
        mv.hits = Some(vec![Hit {
            frames: (15, 10), // End before start
            damage: 50,
            chip_damage: None,
            hitstun: 20,
            blockstun: 14,
            hitstop: 8,
            guard: GuardType::Mid,
            hitboxes: vec![],
            cancels: vec![],
        }]);
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "hits[0].frames"));
    }

    #[test]
    fn test_hitbox_shape_zero_dimension_fails() {
        let mut mv = make_valid_move();
        mv.hits = Some(vec![Hit {
            frames: (10, 15),
            damage: 50,
            chip_damage: None,
            hitstun: 20,
            blockstun: 14,
            hitstop: 8,
            guard: GuardType::Mid,
            hitboxes: vec![
                HitboxShape::Aabb {
                    x: 10,
                    y: -40,
                    w: 0, // Invalid: zero width
                    h: 20,
                },
                HitboxShape::Circle {
                    x: 50,
                    y: -30,
                    r: 0, // Invalid: zero radius
                },
            ],
            cancels: vec![],
        }]);
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "hits[0].hitboxes[0].w"));
        assert!(errors.iter().any(|e| e.field == "hits[0].hitboxes[1].r"));
    }

    #[test]
    fn test_precondition_meter_range_fails() {
        let mut mv = make_valid_move();
        mv.preconditions = Some(vec![Precondition::Meter {
            min: Some(100),
            max: Some(50), // Invalid: min > max
        }]);
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "preconditions[0]"
            && e.message.contains("meter min cannot be greater than max")));
    }

    #[test]
    fn test_precondition_charge_zero_frames_fails() {
        let mut mv = make_valid_move();
        mv.preconditions = Some(vec![Precondition::Charge {
            direction: "4".to_string(),
            min_frames: 0, // Invalid: zero frames
        }]);
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "preconditions[0].min_frames"));
    }

    #[test]
    fn test_precondition_health_range_fails() {
        let mut mv = make_valid_move();
        mv.preconditions = Some(vec![Precondition::Health {
            min_percent: Some(80),
            max_percent: Some(30), // Invalid: min > max
        }]);
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "preconditions[0]"
            && e.message.contains("min_percent cannot be greater")));
    }

    #[test]
    fn test_precondition_health_over_100_fails() {
        let mut mv = make_valid_move();
        mv.preconditions = Some(vec![Precondition::Health {
            min_percent: Some(150), // Invalid: > 100
            max_percent: None,
        }]);
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("cannot exceed 100")));
    }

    #[test]
    fn test_cost_zero_amount_fails() {
        let mut mv = make_valid_move();
        mv.costs = Some(vec![Cost::Meter { amount: 0 }]);
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "costs[0].amount" && e.message.contains("must be greater than 0")));
    }

    #[test]
    fn test_movement_zero_distance_fails() {
        let mut mv = make_valid_move();
        mv.movement = Some(Movement {
            distance: Some(0), // Invalid: zero distance
            direction: Some("forward".to_string()),
            curve: None,
            airborne: None,
            velocity: None,
            acceleration: None,
            frames: None,
        });
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "movement.distance"));
    }

    #[test]
    fn test_movement_missing_velocity_and_distance_fails() {
        let mut mv = make_valid_move();
        mv.movement = Some(Movement {
            distance: None,
            direction: None,
            curve: None,
            airborne: None,
            velocity: None, // Neither distance nor velocity is set
            acceleration: None,
            frames: None,
        });
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "movement"
            && e.message.contains("must have either distance or velocity")));
    }

    #[test]
    fn test_super_freeze_zero_frames_fails() {
        let mut mv = make_valid_move();
        mv.super_freeze = Some(SuperFreeze {
            frames: 0, // Invalid: zero frames
            zoom: None,
            darken: None,
            flash: None,
        });
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "super_freeze.frames"));
    }

    #[test]
    fn test_super_freeze_invalid_zoom_fails() {
        let mut mv = make_valid_move();
        mv.super_freeze = Some(SuperFreeze {
            frames: 45,
            zoom: Some(-1.0), // Invalid: negative zoom
            darken: None,
            flash: None,
        });
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "super_freeze.zoom"));
    }

    #[test]
    fn test_super_freeze_invalid_darken_fails() {
        let mut mv = make_valid_move();
        mv.super_freeze = Some(SuperFreeze {
            frames: 45,
            zoom: None,
            darken: Some(1.5), // Invalid: > 1.0
            flash: None,
        });
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.field == "super_freeze.darken"
                    && e.message.contains("between 0.0 and 1.0"))
        );
    }

    #[test]
    fn test_status_effect_zero_duration_fails() {
        let mut mv = make_valid_move();
        mv.on_hit = Some(OnHit {
            gain_meter: None,
            heal: None,
            status: Some(vec![StatusEffect::Stun { duration: 0 }]),
            knockback: None,
            wall_bounce: None,
            ground_bounce: None,
            events: vec![],
            resource_deltas: vec![],
        });
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "on_hit.status[0].duration"));
    }

    #[test]
    fn test_status_effect_poison_zero_damage_fails() {
        let mut mv = make_valid_move();
        mv.on_hit = Some(OnHit {
            gain_meter: None,
            heal: None,
            status: Some(vec![StatusEffect::Poison {
                damage_per_frame: 0, // Invalid
                duration: 120,
            }]),
            knockback: None,
            wall_bounce: None,
            ground_bounce: None,
            events: vec![],
            resource_deltas: vec![],
        });
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "on_hit.status[0].damage_per_frame"));
    }

    #[test]
    fn test_status_effect_slow_invalid_multiplier_fails() {
        let mut mv = make_valid_move();
        mv.on_hit = Some(OnHit {
            gain_meter: None,
            heal: None,
            status: Some(vec![StatusEffect::Slow {
                multiplier: 1.5, // Invalid: > 1.0
                duration: 60,
            }]),
            knockback: None,
            wall_bounce: None,
            ground_bounce: None,
            events: vec![],
            resource_deltas: vec![],
        });
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "on_hit.status[0].multiplier"
                && e.message.contains("between 0.0 and 1.0")));
    }

    #[test]
    fn test_frame_hurtbox_frame_order_fails() {
        let mut mv = make_valid_move();
        mv.advanced_hurtboxes = Some(vec![FrameHurtbox {
            frames: (20, 10), // Invalid: end before start
            boxes: vec![HitboxShape::Aabb {
                x: -15,
                y: -70,
                w: 30,
                h: 70,
            }],
            flags: None,
        }]);
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "advanced_hurtboxes[0].frames"));
    }

    #[test]
    fn test_frame_hurtbox_invalid_shape_fails() {
        let mut mv = make_valid_move();
        mv.advanced_hurtboxes = Some(vec![FrameHurtbox {
            frames: (0, 20),
            boxes: vec![HitboxShape::Capsule {
                x1: 0,
                y1: -40,
                x2: 60,
                y2: -30,
                r: 0, // Invalid: zero radius
            }],
            flags: None,
        }]);
        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.field == "advanced_hurtboxes[0].boxes[0].r"));
    }

    #[test]
    fn test_valid_v2_move_with_all_fields_passes() {
        let mut mv = make_valid_move();

        // Add valid v2 fields
        mv.hits = Some(vec![Hit {
            frames: (7, 9),
            damage: 30,
            chip_damage: Some(3),
            hitstun: 15,
            blockstun: 10,
            hitstop: 6,
            guard: GuardType::Mid,
            hitboxes: vec![HitboxShape::Aabb {
                x: 10,
                y: -40,
                w: 30,
                h: 20,
            }],
            cancels: vec!["5M".to_string()],
        }]);

        mv.preconditions = Some(vec![
            Precondition::Meter {
                min: Some(25),
                max: None,
            },
            Precondition::Grounded,
        ]);

        mv.costs = Some(vec![Cost::Meter { amount: 25 }]);

        mv.movement = Some(Movement {
            distance: Some(80),
            direction: Some("forward".to_string()),
            curve: Some("ease-out".to_string()),
            airborne: Some(false),
            velocity: None,
            acceleration: None,
            frames: None,
        });

        mv.super_freeze = Some(SuperFreeze {
            frames: 45,
            zoom: Some(1.5),
            darken: Some(0.7),
            flash: Some(true),
        });

        mv.on_hit = Some(OnHit {
            gain_meter: Some(25),
            heal: None,
            status: Some(vec![StatusEffect::Poison {
                damage_per_frame: 1,
                duration: 120,
            }]),
            knockback: None,
            wall_bounce: None,
            ground_bounce: None,
            events: vec![],
            resource_deltas: vec![],
        });

        mv.advanced_hurtboxes = Some(vec![FrameHurtbox {
            frames: (0, 18),
            boxes: vec![HitboxShape::Aabb {
                x: -15,
                y: -70,
                w: 30,
                h: 70,
            }],
            flags: Some(vec![crate::schema::HurtboxFlag::StrikeInvuln]),
        }]);

        assert!(validate_move(&mv).is_ok());
    }

    #[test]
    fn test_multiple_validation_errors() {
        let mut mv = make_valid_move();
        mv.startup = 0;
        mv.input = "".to_string();
        mv.super_freeze = Some(SuperFreeze {
            frames: 0,
            zoom: Some(-1.0),
            darken: Some(2.0),
            flash: None,
        });

        let result = validate_move(&mv);
        assert!(result.is_err());
        let errors = result.unwrap_err();

        // Should have at least 4 errors: startup, input, super_freeze.frames, super_freeze.zoom, super_freeze.darken
        assert!(errors.len() >= 4);
        assert!(errors.iter().any(|e| e.field == "startup"));
        assert!(errors.iter().any(|e| e.field == "input"));
        assert!(errors.iter().any(|e| e.field == "super_freeze.frames"));
    }
}
