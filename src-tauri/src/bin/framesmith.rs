//! Framesmith CLI
//!
//! Minimal CLI intended for automation (e.g. exporting .fspk packs).
//!
//! Examples:
//!   cargo run --bin framesmith -- export --project .. --character test_char --out ..\\exports\\test_char.fspk
//!   cargo run --bin framesmith -- export --project .. --all --out-dir ..\\exports

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Adapter {
    ZxFspack,
    JsonBlob,
}

impl Default for Adapter {
    fn default() -> Self {
        Self::ZxFspack
    }
}

impl Adapter {
    fn parse(s: &str) -> Result<Self, String> {
        match s {
            "zx-fspack" => Ok(Self::ZxFspack),
            "json-blob" => Ok(Self::JsonBlob),
            _ => Err(format!("Unknown adapter: {}", s)),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::ZxFspack => "zx-fspack",
            Self::JsonBlob => "json-blob",
        }
    }

    fn default_ext(self) -> &'static str {
        match self {
            Self::ZxFspack => ".fspk",
            Self::JsonBlob => ".json",
        }
    }
}

#[derive(Debug, Clone, Default)]
struct ExportArgs {
    project_dir: Option<PathBuf>,
    characters_dir: Option<PathBuf>,
    character_id: Option<String>,
    all: bool,
    out: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    adapter: Adapter,
    pretty: bool,
    keep_going: bool,
}

fn usage() -> &'static str {
    "Framesmith CLI\n\nUSAGE:\n  framesmith export [options]\n\nOPTIONS:\n  --project <dir>         Project root (expects <dir>/characters)\n  --characters-dir <dir>  Characters directory (overrides --project)\n  --character <id>        Character ID (folder name under characters dir)\n  --all                   Export all characters\n  --out <file>            Output file (single-character export)\n  --out-dir <dir>         Output directory (export all)\n  --adapter <name>        Adapter: zx-fspack (default), json-blob\n  --pretty                Pretty JSON output (json-blob only)\n  --keep-going            Continue exporting others after an error (export all only)\n  -h, --help              Print help\n\nENV:\n  FRAMESMITH_CHARACTERS_DIR  Default characters directory if not provided\n"
}

fn main() {
    if let Err(e) = real_main() {
        eprintln!("error: {}", e);
        process::exit(1);
    }
}

fn real_main() -> Result<(), String> {
    let argv: Vec<String> = env::args().skip(1).collect();
    if argv.is_empty() {
        print!("{}", usage());
        return Ok(());
    }

    match argv[0].as_str() {
        "-h" | "--help" => {
            print!("{}", usage());
            Ok(())
        }
        "export" => cmd_export(&argv[1..]),
        other => Err(format!("Unknown command: {}\n\n{}", other, usage())),
    }
}

fn cmd_export(args: &[String]) -> Result<(), String> {
    let mut cfg = ExportArgs {
        adapter: Adapter::ZxFspack,
        ..Default::default()
    };

    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "-h" | "--help" => {
                print!("{}", usage());
                return Ok(());
            }
            "--project" => {
                i += 1;
                cfg.project_dir = Some(PathBuf::from(arg_value(args, i, "--project")?));
            }
            "--characters-dir" => {
                i += 1;
                cfg.characters_dir = Some(PathBuf::from(arg_value(args, i, "--characters-dir")?));
            }
            "--character" => {
                i += 1;
                cfg.character_id = Some(arg_value(args, i, "--character")?.to_string());
            }
            "--all" => {
                cfg.all = true;
            }
            "--out" => {
                i += 1;
                cfg.out = Some(PathBuf::from(arg_value(args, i, "--out")?));
            }
            "--out-dir" => {
                i += 1;
                cfg.out_dir = Some(PathBuf::from(arg_value(args, i, "--out-dir")?));
            }
            "--adapter" => {
                i += 1;
                cfg.adapter = Adapter::parse(arg_value(args, i, "--adapter")?)?;
            }
            "--pretty" => {
                cfg.pretty = true;
            }
            "--keep-going" => {
                cfg.keep_going = true;
            }
            _ => {
                return Err(format!("Unknown option: {}\n\n{}", a, usage()));
            }
        }
        i += 1;
    }

    if cfg.project_dir.is_some() && cfg.characters_dir.is_some() {
        return Err("Provide only one of --project or --characters-dir".to_string());
    }

    if cfg.all == cfg.character_id.is_some() {
        return Err("Provide exactly one of --all or --character <id>".to_string());
    }

    if cfg.all && cfg.out.is_some() {
        return Err("--out cannot be used with --all (use --out-dir)".to_string());
    }

    if !cfg.all && cfg.out_dir.is_some() {
        return Err("--out-dir cannot be used with --character (use --out)".to_string());
    }

    if cfg.adapter == Adapter::ZxFspack && cfg.pretty {
        return Err("--pretty is only supported for json-blob".to_string());
    }

    let characters_dir =
        resolve_characters_dir(cfg.project_dir.as_deref(), cfg.characters_dir.as_deref())?;
    if !characters_dir.exists() {
        return Err(format!(
            "Characters directory does not exist: {}",
            characters_dir.display()
        ));
    }

    if cfg.all {
        export_all(&characters_dir, &cfg)
    } else {
        let id = cfg.character_id.clone().expect("character_id checked");
        export_one(&characters_dir, &id, &cfg)
    }
}

fn arg_value<'a>(args: &'a [String], i: usize, flag: &str) -> Result<&'a str, String> {
    args.get(i)
        .map(|s| s.as_str())
        .ok_or_else(|| format!("Missing value for {}", flag))
}

fn resolve_characters_dir(
    project_dir: Option<&Path>,
    characters_dir: Option<&Path>,
) -> Result<PathBuf, String> {
    if let Some(dir) = characters_dir {
        return Ok(dir.to_path_buf());
    }
    if let Some(project) = project_dir {
        return Ok(project.join("characters"));
    }

    if let Ok(env_dir) = env::var("FRAMESMITH_CHARACTERS_DIR") {
        if !env_dir.trim().is_empty() {
            return Ok(PathBuf::from(env_dir));
        }
    }

    Ok(PathBuf::from("./characters"))
}

fn default_out_dir(project_dir: Option<&Path>) -> PathBuf {
    match project_dir {
        Some(project) => project.join("exports"),
        None => PathBuf::from("./exports"),
    }
}

fn export_one(characters_dir: &Path, character_id: &str, cfg: &ExportArgs) -> Result<(), String> {
    let out = match &cfg.out {
        Some(p) => p.clone(),
        None => {
            let base = default_out_dir(cfg.project_dir.as_deref());
            base.join(format!("{}{}", character_id, cfg.adapter.default_ext()))
        }
    };

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create output directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    d_developmentnethercore_projectframesmith_lib::commands::export_character(
        characters_dir.to_string_lossy().to_string(),
        character_id.to_string(),
        cfg.adapter.as_str().to_string(),
        out.to_string_lossy().to_string(),
        cfg.pretty,
    )?;

    println!("Exported {} -> {}", character_id, out.display());
    Ok(())
}

fn export_all(characters_dir: &Path, cfg: &ExportArgs) -> Result<(), String> {
    let out_dir = match &cfg.out_dir {
        Some(p) => p.clone(),
        None => default_out_dir(cfg.project_dir.as_deref()),
    };

    fs::create_dir_all(&out_dir).map_err(|e| {
        format!(
            "Failed to create output directory {}: {}",
            out_dir.display(),
            e
        )
    })?;

    let ids = find_character_ids(characters_dir)?;
    if ids.is_empty() {
        return Err(format!(
            "No characters found under {}",
            characters_dir.display()
        ));
    }

    let mut failures: Vec<String> = Vec::new();
    for id in ids {
        let out = out_dir.join(format!("{}{}", id, cfg.adapter.default_ext()));
        match export_one(
            characters_dir,
            &id,
            &ExportArgs {
                out: Some(out),
                ..cfg.clone()
            },
        ) {
            Ok(()) => {}
            Err(e) => {
                let msg = format!("{}: {}", id, e);
                eprintln!("{}", msg);
                failures.push(msg);
                if !cfg.keep_going {
                    break;
                }
            }
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!("Export failed: {}", failures.join("; ")))
    }
}

fn find_character_ids(characters_dir: &Path) -> Result<Vec<String>, String> {
    let mut ids: Vec<String> = Vec::new();
    let rd = fs::read_dir(characters_dir).map_err(|e| {
        format!(
            "Failed to read characters directory {}: {}",
            characters_dir.display(),
            e
        )
    })?;

    for entry in rd {
        let entry =
            entry.map_err(|e| format!("Failed to read characters directory entry: {}", e))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !path.join("character.json").exists() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => continue,
        };
        ids.push(name);
    }

    ids.sort();
    ids.dedup();
    Ok(ids)
}
