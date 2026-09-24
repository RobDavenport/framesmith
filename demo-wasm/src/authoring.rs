use framesmith_authoring::{codegen, globals, rules, schema, variant};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

pub type Files = BTreeMap<String, Value>;
const ROOT: &str = "characters/relay/";

pub fn baseline() -> Files {
    serde_json::from_str(include_str!("../dist/lab-source.json")).expect("CLI-built source bundle")
}
fn read<T: serde::de::DeserializeOwned>(files: &Files, name: &str) -> Result<T, String> {
    serde_json::from_value(
        files
            .get(name)
            .ok_or_else(|| format!("Missing {name}"))?
            .clone(),
    )
    .map_err(|e| e.to_string())
}
pub fn compile(files: &Files) -> Result<Vec<u8>, String> {
    let character: schema::Character = read(files, &format!("{ROOT}character.json"))?;
    let table = read(files, &format!("{ROOT}cancel_table.json"))?;
    let registry: rules::RulesFile = read(files, "framesmith.rules.json")?;
    if registry.version != 1 {
        return Err("Unsupported rules version".into());
    }
    let mut named: Vec<(String, schema::State)> = Vec::new();
    for (path, value) in files {
        if let Some(name) = path
            .strip_prefix(&format!("{ROOT}states/"))
            .and_then(|p| p.strip_suffix(".json"))
        {
            named.push((
                name.into(),
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?,
            ));
        }
    }
    let manifest: schema::GlobalsManifest = read(files, &format!("{ROOT}globals.json"))?;
    let local: HashSet<String> = named.iter().map(|(_, s)| s.input.clone()).collect();
    let (shared, _) = globals::resolve_global_manifest(&manifest, &local, |name| {
        read(files, &format!("globals/states/{name}.json")).map_err(|e| {
            globals::GlobalsError::ParseError {
                path: name.into(),
                message: e,
            }
        })
    })
    .map_err(|e| e.to_string())?;
    named.extend(shared.into_iter().map(|s| (s.input.clone(), s)));
    let states = variant::flatten_variants(named)?;
    let data = codegen::prepare_character(character, states, table, Some(&registry), None)?;
    codegen::export_fspk(
        &data,
        Some(&rules::MergedRules::merge(Some(&registry), None)),
    )
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub recovery: u8,
    pub follow_startup: u8,
    pub cancel: bool,
    pub condition: String,
    pub window_start: u8,
    pub window_end: u8,
    pub deny: bool,
    pub tagged: bool,
    pub energy: u16,
    pub gain: i32,
    pub cost: u16,
    pub ammo: u16,
    pub reach: u32,
    pub notify_frame: u16,
    pub spark_size: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            recovery: 12,
            follow_startup: 7,
            cancel: true,
            condition: "hit".into(),
            window_start: 0,
            window_end: 255,
            deny: false,
            tagged: true,
            energy: 0,
            gain: 20,
            cost: 50,
            ammo: 3,
            reach: 70,
            notify_frame: 5,
            spark_size: 18.,
        }
    }
}
impl Settings {
    pub fn edit(&mut self, key: &str, value: &str) -> Result<(), String> {
        if value.len() > 64 {
            return Err("Setting is too large".into());
        }
        let v: Value = serde_json::from_str(value).map_err(|e| e.to_string())?;
        let number = |lo: u64, hi: u64| -> Result<u64, String> {
            let x = v.as_u64().ok_or("Use a whole nonnegative number")?;
            if !(lo..=hi).contains(&x) {
                return Err(format!("{key} must be in {lo}..={hi}"));
            }
            Ok(x)
        };
        let boolean = || {
            v.as_bool()
                .ok_or_else(|| String::from("Expected a checkbox value"))
        };
        match key {
            "recovery" => self.recovery = number(0, 30)? as u8,
            "follow_startup" => self.follow_startup = number(2, 20)? as u8,
            "cancel" => self.cancel = boolean()?,
            "deny" => self.deny = boolean()?,
            "tagged" => self.tagged = boolean()?,
            "condition" => {
                let s = v.as_str().ok_or("Expected a condition")?;
                if !["hit", "block", "whiff", "always"].contains(&s) {
                    return Err("Unknown cancel condition".into());
                }
                self.condition = s.into();
            }
            "window_start" => self.window_start = number(0, 60)? as u8,
            "window_end" => self.window_end = number(0, 255)? as u8,
            "energy" => self.energy = number(0, 100)? as u16,
            "gain" => self.gain = number(0, 50)? as i32,
            "cost" => self.cost = number(0, 100)? as u16,
            "ammo" => self.ammo = number(0, 3)? as u16,
            "reach" => self.reach = number(20, 140)? as u32,
            "notify_frame" => self.notify_frame = number(0, 30)? as u16,
            "spark_size" => self.spark_size = number(4, 48)? as f64,
            _ => return Err(format!("Unknown setting: {key}")),
        }
        if self.window_start > self.window_end {
            return Err("Cancel window starts after it ends".into());
        }
        Ok(())
    }
    pub fn files(&self) -> Files {
        let mut f = baseline();
        let key = |s: &str| format!("{ROOT}states/{s}.json");
        let jab = f.get_mut(&key("jab")).unwrap();
        jab["recovery"] = self.recovery.into();
        jab["hitboxes"][0]["box"]["w"] = self.reach.into();
        jab["notifies"][0]["frame"] = self.notify_frame.into();
        for k in ["jab", "follow"] {
            let s = f.get_mut(&key(k)).unwrap();
            if k == "follow" {
                let shift = i64::from(self.follow_startup) - 7;
                s["startup"] = self.follow_startup.into();
                for w in s["hitboxes"].as_array_mut().unwrap() {
                    for frame in w["frames"].as_array_mut().unwrap() {
                        *frame = (frame.as_i64().unwrap() + shift).into();
                    }
                }
                s["notifies"][0]["frame"] = self.follow_startup.into();
            }
            if !self.tagged {
                s["tags"] = json!(["attack", "unlinked"]);
            }
        }
        for k in ["jab", "follow", "special", "multi"] {
            f.get_mut(&key(k)).unwrap()["on_hit"]["resource_deltas"][0]["delta"] = self.gain.into();
        }
        for k in ["jab", "follow", "special", "finisher", "multi"] {
            let s = f.get_mut(&key(k)).unwrap();
            // Event size is per-event; the independent character property scales presentation.
            let total = s["startup"].as_u64().unwrap()
                + s["active"].as_u64().unwrap()
                + s["recovery"].as_u64().unwrap();
            s["hurtboxes"][0]["frames"][1] = (total - 1).into();
            s["pushboxes"][0]["frames"][1] = (total - 1).into();
        }
        let fin = f.get_mut(&key("finisher")).unwrap();
        fin["costs"][0]["amount"] = self.cost.into();
        fin["preconditions"][0]["min"] = self.cost.into();
        let char = f.get_mut(&format!("{ROOT}character.json")).unwrap();
        char["resources"][0]["start"] = self.energy.into();
        char["resources"][1]["start"] = self.ammo.into();
        char["properties"]["spark_size"] = self.spark_size.into();
        let table = f.get_mut(&format!("{ROOT}cancel_table.json")).unwrap();
        table["tag_rules"][1]["on"] = self.condition.clone().into();
        table["tag_rules"][1]["after_frame"] = self.window_start.into();
        table["tag_rules"][1]["before_frame"] = self.window_end.into();
        if !self.cancel {
            table["tag_rules"].as_array_mut().unwrap().remove(1);
        }
        if self.deny {
            table["deny"] = json!({"follow":["special","special~charged"]});
        }
        f
    }
}
