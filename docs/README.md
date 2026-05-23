# Framesmith Documentation

Status: active
Last reviewed: 2026-05-23

## Source of truth map

| Topic | Location |
|-------|----------|
| Repo constraints and invariants | [`../CLAUDE.md`](../CLAUDE.md) |
| Contributor and code-map reference | [`../AGENTS.md`](../AGENTS.md) |
| Project overview and quick start | [`../README.md`](../README.md) |
| Data formats (on-disk JSON) | [`data-formats.md`](data-formats.md) |
| Rules semantics (SSOT) | [`rules-spec.md`](rules-spec.md) |
| MCP server | [`mcp-server.md`](mcp-server.md) |
| ZX FSPK format | [`zx-fspack.md`](zx-fspack.md) |
| Runtime integration | [`runtime-guide.md`](runtime-guide.md) |
| Runtime API | [`runtime-api.md`](runtime-api.md) |
| CLI usage | [`cli.md`](cli.md) |
| Production handoff decision | [`production-handoff-decision.md`](production-handoff-decision.md) |
| Variant editing decision | [`variant-editing-decision.md`](variant-editing-decision.md) |
| Export fidelity contract | [`export-fidelity-contract.md`](export-fidelity-contract.md) |
| Combat mechanic coverage | [`combat-coverage.md`](combat-coverage.md) |
| Training scenario contract | [`training-scenario-contract.md`](training-scenario-contract.md) |
| Production gap backlog | [`production-gap-backlog.md`](production-gap-backlog.md) |
| Release runbook | [`release-runbook.md`](release-runbook.md) |
| Branch protection setup | [`branch-protection-setup.md`](branch-protection-setup.md) |
| Current release evidence | [`release-evidence-2026-05-23.md`](release-evidence-2026-05-23.md) |
| Schema migration notes | [`schema-migration.md`](schema-migration.md) |
| Windows installer smoke test | [`windows-installer-smoke-test.md`](windows-installer-smoke-test.md) |
| Global states | [`global-states.md`](global-states.md) |
| Architecture overview | [`architecture.md`](architecture.md) |
| Troubleshooting | [`troubleshooting.md`](troubleshooting.md) |
| Character authoring flow | [`character-authoring-guide.md`](character-authoring-guide.md) |
| Movement reference | [`movement-reference.md`](movement-reference.md) |
| Implementation history | [`implementation-history.md`](implementation-history.md) |
| Production readiness | [`production-readiness-plan.md`](production-readiness-plan.md) |

## Reading order

- New to Framesmith: start with [`../README.md`](../README.md)
- Editing schema or files on disk: read [`data-formats.md`](data-formats.md)
- Changing validation/rules behavior: read [`rules-spec.md`](rules-spec.md)
- Integrating external tools/LLMs: read [`mcp-server.md`](mcp-server.md)
- Implementing export/runtime work: read [`production-handoff-decision.md`](production-handoff-decision.md), [`variant-editing-decision.md`](variant-editing-decision.md), [`combat-coverage.md`](combat-coverage.md), [`training-scenario-contract.md`](training-scenario-contract.md), [`export-fidelity-contract.md`](export-fidelity-contract.md), [`zx-fspack.md`](zx-fspack.md), and [`runtime-guide.md`](runtime-guide.md)
- Understanding the system architecture: read [`architecture.md`](architecture.md)
- Debugging issues: read [`troubleshooting.md`](troubleshooting.md)
- Tracking release blockers: read [`production-readiness-plan.md`](production-readiness-plan.md), [`production-gap-backlog.md`](production-gap-backlog.md), [`release-runbook.md`](release-runbook.md), and [`branch-protection-setup.md`](branch-protection-setup.md)
- Migrating older project data: read [`schema-migration.md`](schema-migration.md)
- Testing Windows release artifacts: read [`windows-installer-smoke-test.md`](windows-installer-smoke-test.md)

## Document set

| Document | Purpose |
|----------|---------|
| [`data-formats.md`](data-formats.md) | Canonical file layout and JSON data model |
| [`rules-spec.md`](rules-spec.md) | Rule matching, defaults, and validation behavior |
| [`mcp-server.md`](mcp-server.md) | MCP server setup and available operations |
| [`zx-fspack.md`](zx-fspack.md) | Binary pack format details |
| [`runtime-guide.md`](runtime-guide.md) | Runtime integration path |
| [`runtime-api.md`](runtime-api.md) | Runtime API reference |
| [`cli.md`](cli.md) | CLI commands and examples |
| [`production-handoff-decision.md`](production-handoff-decision.md) | Canonical production handoff and FSPK v1 movement policy |
| [`variant-editing-decision.md`](variant-editing-decision.md) | Variant overlay editing policy for the first production target |
| [`export-fidelity-contract.md`](export-fidelity-contract.md) | Adapter field-preservation contract and known FSPK limits |
| [`combat-coverage.md`](combat-coverage.md) | Fighting-game mechanic support and engine-owned gaps |
| [`training-scenario-contract.md`](training-scenario-contract.md) | Executable target training scenarios and ownership policy |
| [`production-gap-backlog.md`](production-gap-backlog.md) | Concrete implementation issues for external gates and future target-game gaps |
| [`release-runbook.md`](release-runbook.md) | Repeatable release-candidate validation and evidence capture steps |
| [`branch-protection-setup.md`](branch-protection-setup.md) | Exact required CI branch-protection settings and evidence template |
| [`release-evidence-2026-05-23.md`](release-evidence-2026-05-23.md) | Current candidate validation evidence and external release blockers |
| [`schema-migration.md`](schema-migration.md) | Migration notes for current character, cancel, variant, and adapter schema changes |
| [`windows-installer-smoke-test.md`](windows-installer-smoke-test.md) | Manual MSI/NSIS smoke-test steps and evidence to record |
| [`global-states.md`](global-states.md) | Global state model and usage |
| [`character-authoring-guide.md`](character-authoring-guide.md) | Authoring workflow guidance |
| [`movement-reference.md`](movement-reference.md) | Movement and notation reference |
| [`architecture.md`](architecture.md) | System architecture and data pipeline overview |
| [`troubleshooting.md`](troubleshooting.md) | Common issues and solutions |
| [`design.md`](design.md) | Design rationale and roadmap notes |
| [`implementation-history.md`](implementation-history.md) | Archive index for completed temporary plans |
| [`production-readiness-plan.md`](production-readiness-plan.md) | Production readiness blockers, acceptance criteria, and checklist |

## Plans

There are no active checked-in implementation plans. Completed temporary plans
were migrated into permanent docs and summarized in
[`implementation-history.md`](implementation-history.md).
