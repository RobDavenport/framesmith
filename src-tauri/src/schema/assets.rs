use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum AnimationClip {
    Sprite {
        texture: String,
        frame_size: FrameSize,
        frames: u32,
        #[serde(default)]
        pivot: Pivot2,
    },
    Gltf {
        model: String,
        clip: String,
        #[serde(default = "default_clip_fps")]
        fps: f32,
        #[serde(default)]
        pivot: Pivot3,
    },
}

fn default_clip_fps() -> f32 {
    60.0
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FrameSize {
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Pivot2 {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(default)]
pub struct Pivot3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for Pivot3 {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_clip_deserializes() {
        let clip: AnimationClip = serde_json::from_str(
            r#"{
              "mode": "sprite",
              "texture": "atlas.main",
              "frame_size": { "w": 64, "h": 32 },
              "frames": 18,
              "pivot": { "x": 128, "y": 192 }
            }"#,
        )
        .expect("sprite clip should parse");

        match clip {
            AnimationClip::Sprite {
                texture,
                frame_size,
                frames,
                pivot,
            } => {
                assert_eq!(texture, "atlas.main");
                assert_eq!(frame_size.w, 64);
                assert_eq!(frame_size.h, 32);
                assert_eq!(frames, 18);
                assert_eq!(pivot.x, 128);
                assert_eq!(pivot.y, 192);
            }
            _ => panic!("expected sprite variant"),
        }
    }

    #[test]
    fn gltf_clip_deserializes_with_default_fps_and_pivot() {
        let clip: AnimationClip = serde_json::from_str(
            r#"{
              "mode": "gltf",
              "model": "char.body",
              "clip": "Idle"
            }"#,
        )
        .expect("gltf clip should parse");

        match clip {
            AnimationClip::Gltf {
                model,
                clip,
                fps,
                pivot,
            } => {
                assert_eq!(model, "char.body");
                assert_eq!(clip, "Idle");
                assert!((fps - 60.0).abs() < 1e-6);
                assert!((pivot.x - 0.0).abs() < 1e-6);
                assert!((pivot.y - 0.0).abs() < 1e-6);
                assert!((pivot.z - 0.0).abs() < 1e-6);
            }
            _ => panic!("expected gltf variant"),
        }
    }

    #[test]
    fn pivots_allow_partial_objects() {
        let sprite: AnimationClip = serde_json::from_str(
            r#"{
              "mode": "sprite",
              "texture": "atlas.main",
              "frame_size": { "w": 64, "h": 32 },
              "frames": 2,
              "pivot": { "x": 10 }
            }"#,
        )
        .expect("sprite clip with partial pivot should parse");

        match sprite {
            AnimationClip::Sprite { pivot, .. } => {
                assert_eq!(pivot.x, 10);
                assert_eq!(pivot.y, 0);
            }
            _ => panic!("expected sprite variant"),
        }

        let gltf: AnimationClip = serde_json::from_str(
            r#"{
              "mode": "gltf",
              "model": "char.body",
              "clip": "Idle",
              "pivot": { "z": 2.0 }
            }"#,
        )
        .expect("gltf clip with partial pivot should parse");

        match gltf {
            AnimationClip::Gltf { pivot, .. } => {
                assert!((pivot.x - 0.0).abs() < 1e-6);
                assert!((pivot.y - 0.0).abs() < 1e-6);
                assert!((pivot.z - 2.0).abs() < 1e-6);
            }
            _ => panic!("expected gltf variant"),
        }
    }

    #[test]
    fn unknown_mode_errors() {
        let err = serde_json::from_str::<AnimationClip>(r#"{ "mode": "unknown", "texture": "x" }"#)
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown"));
    }
}
