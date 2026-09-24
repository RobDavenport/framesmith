//! Two engine-owned mechanics using only typed binary views, not a combat schema.
use framesmith_fspack::{payload::builder::encode_pack, PackView};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Input is an authoring value model. encode_pack writes typed binary records,
    // not serialized JSON, and the runtime does not parse JSON or allocate.
    let source = json!({
        "charge_attack": {"windup": 30, "travel": {"x": 80.125, "easing": "engine_curve"},
            "events": [{"frame": 30, "kind": "release", "payload": {"power": 4}}]},
        "ammo_reload": {"starting_ammo": 0,
            "events": [{"frame": 5, "kind": "reload", "amount": 3},
                       {"frame": 10, "kind": "fire", "amount": 1}]}
    });
    let bytes = encode_pack(&source)?;
    let pack = PackView::parse(&bytes)?;
    assert!(
        pack.states().is_none(),
        "generic packs require no fighting-game tables"
    );
    let root = pack.payload().ok_or("missing typed payload")?.root();
    let attack = root.get("charge_attack").ok_or("missing attack")?;
    let ammo = root.get("ammo_reload").ok_or("missing reload")?;
    let mut charge_power = 0;
    let mut ammunition = ammo
        .get("starting_ammo")
        .and_then(|v| v.as_u64())
        .ok_or("invalid ammo")?;
    for frame in 0..=30 {
        for event in attack.get("events").ok_or("missing events")?.children() {
            if event.get("frame").and_then(|v| v.as_u64()) == Some(frame) {
                charge_power = event
                    .get("payload")
                    .and_then(|v| v.get("power"))
                    .and_then(|v| v.as_u64())
                    .ok_or("invalid power")?;
            }
        }
        for event in ammo.get("events").ok_or("missing events")?.children() {
            if event.get("frame").and_then(|v| v.as_u64()) == Some(frame) {
                let amount = event
                    .get("amount")
                    .and_then(|v| v.as_u64())
                    .ok_or("invalid amount")?;
                // This policy belongs to this example, never to PackView.
                match event.get("kind").and_then(|v| v.as_str()) {
                    Some("reload") => {
                        ammunition = ammunition.checked_add(amount).ok_or("overflow")?
                    }
                    Some("fire") => {
                        ammunition = ammunition.checked_sub(amount).ok_or("empty magazine")?
                    }
                    _ => return Err("unknown engine command".into()),
                }
            }
        }
    }
    let travel = attack
        .get("travel")
        .and_then(|v| v.get("x"))
        .and_then(|v| v.as_f64());
    assert_eq!((charge_power, ammunition, travel), (4, 2, Some(80.125)));
    println!("Binary-only policies passed: charge power={charge_power}, ammo={ammunition}, travel={travel:?}");
    Ok(())
}
