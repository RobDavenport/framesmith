const PRODUCTION_PLAN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/production-readiness-plan.md"
));
const DOCS_INDEX: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../docs/README.md"));
const DATA_FORMATS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/data-formats.md"
));
const ARCHITECTURE_DOC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/architecture.md"
));
const VARIANT_DECISION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/variant-editing-decision.md"
));
const IMPLEMENTATION_HISTORY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/implementation-history.md"
));
const SCHEMA_MIGRATION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/schema-migration.md"
));
const TRAINING_SCENARIO_CONTRACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/training-scenario-contract.md"
));
const WINDOWS_INSTALLER_SMOKE_TEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/windows-installer-smoke-test.md"
));
const PRODUCTION_GAP_BACKLOG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/production-gap-backlog.md"
));
const RELEASE_RUNBOOK: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/release-runbook.md"
));
const RELEASE_EVIDENCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../docs/release-evidence-2026-05-23.md"
));
const CI_WORKFLOW: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../.github/workflows/ci.yml"
));
const CHARACTER_COMMANDS: &str = include_str!("../src/commands/character.rs");

#[test]
fn variant_editing_deferral_is_documented_and_linked() {
    for required in [
        "# Variant Editing Decision",
        "Overlay-aware variant editing is explicitly deferred for the first production",
        "Variant overlays remain JSON-authored files",
        "Saving a resolved variant through the State Editor, MCP `update_move`, or",
        "Save only the overlay diff",
        "Resolved variants cannot overwrite base state files or overlay files through",
    ] {
        assert!(
            VARIANT_DECISION.contains(required),
            "variant editing decision should document: {required}"
        );
    }

    for linked_doc in [PRODUCTION_PLAN, DOCS_INDEX, DATA_FORMATS, ARCHITECTURE_DOC] {
        assert!(
            linked_doc.contains("variant-editing-decision.md"),
            "permanent docs should link variant-editing-decision.md"
        );
    }

    assert!(PRODUCTION_PLAN
        .contains("[x] Overlay-aware variant editing is implemented or explicitly deferred for"));
}

#[test]
fn save_move_guard_stays_aligned_with_variant_editing_policy() {
    assert!(CHARACTER_COMMANDS.contains("if let Some(id) = mv.id.as_deref()"));
    assert!(CHARACTER_COMMANDS.contains("if id != mv.input"));
    assert!(CHARACTER_COMMANDS.contains("Resolved variant states are read-only via save_move"));
}

#[test]
fn temporary_plan_docs_are_migrated_or_removed() {
    let plans_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/plans");
    if plans_dir.exists() {
        let remaining_plans: Vec<_> = std::fs::read_dir(&plans_dir)
            .expect("docs/plans should be readable")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "md"))
            .map(|entry| entry.path())
            .collect();

        assert!(
            remaining_plans.is_empty(),
            "completed temporary plans should be migrated or removed: {remaining_plans:?}"
        );
    }

    for required in [
        "# Implementation History",
        "This document replaces completed temporary plans",
        "Variant overlay system",
        "FSPK module refactor and adapter rename",
        "Do not keep completed implementation plans under `docs/plans/`.",
    ] {
        assert!(
            IMPLEMENTATION_HISTORY.contains(required),
            "implementation history should document: {required}"
        );
    }

    assert!(DOCS_INDEX.contains("implementation-history.md"));
    assert!(PRODUCTION_PLAN.contains("[x] Stale temporary plans are migrated or removed."));
}

#[test]
fn schema_migration_notes_are_documented_and_linked() {
    for required in [
        "# Schema Migration Notes",
        "Move them into",
        "`character.properties`",
        "`tag_rules[]` plus `deny`",
        "Resolved variants are read-only editor snapshots.",
        "The `zx-fspack` adapter name remains as a compatibility alias",
        "cargo test --manifest-path src-tauri/Cargo.toml --test export_fidelity_contract",
    ] {
        assert!(
            SCHEMA_MIGRATION.contains(required),
            "schema migration notes should document: {required}"
        );
    }

    assert!(DOCS_INDEX.contains("schema-migration.md"));
    assert!(PRODUCTION_PLAN.contains("Added [`schema-migration.md`](schema-migration.md)"));
}

#[test]
fn training_scenario_contract_is_documented_and_linked() {
    for required in [
        "# Training Scenario Contract",
        "Authored hitstun",
        "Authored blockstun",
        "Resource policy",
        "Throw input policy",
        "Detached training smoke",
        "target_training_fixture_resolves_authored_reaction_states",
        "loads detached training mode through BroadcastChannel sync",
    ] {
        assert!(
            TRAINING_SCENARIO_CONTRACT.contains(required),
            "training scenario contract should document: {required}"
        );
    }

    assert!(DOCS_INDEX.contains("training-scenario-contract.md"));
    assert!(PRODUCTION_PLAN
        .contains("Added [`training-scenario-contract.md`](training-scenario-contract.md)"));
    assert!(PRODUCTION_PLAN
        .contains("[x] Target-game training scenarios cover hitstun/blockstun/resource/throw"));
}

#[test]
fn windows_installer_smoke_test_is_documented_and_linked() {
    for required in [
        "# Windows Installer Smoke Test",
        "Framesmith_<version>_x64_en-US.msi",
        "Framesmith_<version>_x64-setup.exe",
        "Training starts from the packaged WASM and FSPK path.",
        "Evidence To Record",
    ] {
        assert!(
            WINDOWS_INSTALLER_SMOKE_TEST.contains(required),
            "installer smoke test should document: {required}"
        );
    }

    assert!(DOCS_INDEX.contains("windows-installer-smoke-test.md"));
    assert!(PRODUCTION_PLAN.contains("windows-installer-smoke-test.md"));
}

#[test]
fn production_gap_backlog_covers_external_and_target_game_gaps() {
    for required in [
        "# Production Gap Backlog",
        "PROD-CI-001",
        "PROD-CI-002",
        "PROD-WIN-001",
        "FSPK-MOVE-001",
        "FSPK-HIT-001",
        "RUNTIME-THROW-001",
        "RUNTIME-FREEZE-001",
        "RUNTIME-RESOURCE-001",
        "RUNTIME-STAGE-001",
        "RUNTIME-EVENT-001",
        "PLATFORM-LINUX-001",
        "PLATFORM-MAC-001",
    ] {
        assert!(
            PRODUCTION_GAP_BACKLOG.contains(required),
            "production gap backlog should document: {required}"
        );
    }

    for linked_doc in [DOCS_INDEX, PRODUCTION_PLAN, TRAINING_SCENARIO_CONTRACT] {
        assert!(
            linked_doc.contains("production-gap-backlog.md"),
            "permanent docs should link production-gap-backlog.md"
        );
    }
}

#[test]
fn release_runbook_covers_candidate_evidence() {
    for required in [
        "# Release Runbook",
        "package.json",
        "src-tauri/Cargo.toml",
        "src-tauri/tauri.conf.json",
        "npm ci",
        "npm audit",
        "npm ls @tauri-apps/api @tauri-apps/cli @tauri-apps/plugin-opener",
        "Test-Path 'src/lib/wasm/framesmith_runtime_wasm.js'",
        "Test-Path 'src/lib/wasm/framesmith_runtime_wasm.d.ts'",
        "cargo run --manifest-path src-tauri/Cargo.toml --bin framesmith-cli -- export --project . --character test_char --adapter fspk --out exports/test_char.fspk",
        "git diff --exit-code -- schemas/rules.schema.json",
        "GitHub Actions URL",
        "Protected branch/ruleset",
        "windows-installer-smoke-test.md",
        "Final Evidence Template",
    ] {
        assert!(
            RELEASE_RUNBOOK.contains(required),
            "release runbook should document: {required}"
        );
    }

    assert!(DOCS_INDEX.contains("release-runbook.md"));
    assert!(PRODUCTION_PLAN.contains("release-runbook.md"));
    assert!(PRODUCTION_PLAN.contains("[x] Release runbook exists for clean-checkout"));
}

#[test]
fn current_release_evidence_records_external_blockers() {
    for required in [
        "# Release Evidence 2026-05-23",
        "codex-production-readiness-plan",
        "Local validation baseline SHA: 51c3be4c5b5e4d67b093f0f7aaafc96ed244e26d",
        "https://github.com/RobDavenport/framesmith/pull/1",
        "https://github.com/RobDavenport/framesmith/actions/runs/26327908309",
        "CI status: failed",
        "workflow ran `npm run check` before `npm run wasm:build`",
        "workflow now rebuilds the WASM package before frontend type",
        "https://github.com/RobDavenport/framesmith/actions/runs/26328098854",
        "Failing step: Test runtime WASM crate",
        "workflow now exports `characters/test_char` with",
        "MSI result: not manually smoke tested",
        "Decision: not ready",
    ] {
        assert!(
            RELEASE_EVIDENCE.contains(required),
            "release evidence should document: {required}"
        );
    }

    assert!(DOCS_INDEX.contains("release-evidence-2026-05-23.md"));
    assert!(PRODUCTION_PLAN.contains("release-evidence-2026-05-23.md"));
    assert!(PRODUCTION_PLAN.contains("[x] Current candidate release evidence is recorded."));
}

#[test]
fn ci_builds_wasm_before_frontend_typecheck() {
    let rebuild_wasm = CI_WORKFLOW
        .find("name: Rebuild WASM package")
        .expect("CI should rebuild the WASM package");
    let verify_wasm = CI_WORKFLOW
        .find("name: Verify WASM package exists")
        .expect("CI should verify generated WASM bindings exist");
    let typecheck = CI_WORKFLOW
        .find("name: TypeScript and Svelte check")
        .expect("CI should run frontend type checks");

    assert!(
        rebuild_wasm < verify_wasm && verify_wasm < typecheck,
        "clean CI must build ignored WASM bindings before type checking imports"
    );
    assert!(CI_WORKFLOW.contains("Test-Path src/lib/wasm/framesmith_runtime_wasm.js"));
    assert!(CI_WORKFLOW.contains("Test-Path src/lib/wasm/framesmith_runtime_wasm.d.ts"));
}

#[test]
fn ci_generates_runtime_wasm_fixture_before_crate_test() {
    let build_fixture = CI_WORKFLOW
        .find("name: Build runtime WASM test fixture")
        .expect("CI should build the runtime WASM FSPK test fixture");
    let wasm_tests = CI_WORKFLOW
        .find("name: Test runtime WASM crate")
        .expect("CI should run runtime WASM crate tests");

    assert!(
        build_fixture < wasm_tests,
        "clean CI must generate ignored exports/test_char.fspk before WASM crate tests"
    );
    assert!(CI_WORKFLOW.contains("--bin framesmith-cli -- export --project . --character test_char --adapter fspk --out exports/test_char.fspk"));
}
