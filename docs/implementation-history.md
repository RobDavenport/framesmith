# Implementation History

Status: archived
Last reviewed: 2026-05-23

This document replaces completed temporary plans that previously lived under
`docs/plans/`. It is not a source of truth for current behavior. Use the linked
permanent documents and tests for normative details.

## Migrated Plans

| Historical plan | Durable home |
|-----------------|--------------|
| Framesmith runtime scaffold and phase 2 runtime work | [`runtime-guide.md`](runtime-guide.md), [`runtime-api.md`](runtime-api.md), [`combat-coverage.md`](combat-coverage.md) |
| Training mode design | [`runtime-guide.md`](runtime-guide.md), [`troubleshooting.md`](troubleshooting.md), [`production-readiness-plan.md`](production-readiness-plan.md) |
| Training/rendercore unification | [`architecture.md`](architecture.md), `src/lib/rendercore/`, `src/lib/training/` tests |
| State tags, cancel rules, and move-to-state terminology | [`data-formats.md`](data-formats.md), [`rules-spec.md`](rules-spec.md), [`zx-fspack.md`](zx-fspack.md), [`runtime-guide.md`](runtime-guide.md) |
| Variant overlay system | [`data-formats.md`](data-formats.md), [`variant-editing-decision.md`](variant-editing-decision.md), [`architecture.md`](architecture.md) |
| Global states | [`global-states.md`](global-states.md), [`data-formats.md`](data-formats.md), [`mcp-server.md`](mcp-server.md) |
| FSPK character properties and pushboxes | [`zx-fspack.md`](zx-fspack.md), [`runtime-guide.md`](runtime-guide.md), [`runtime-api.md`](runtime-api.md), [`export-fidelity-contract.md`](export-fidelity-contract.md) |
| Cancel condition bitfield | [`data-formats.md`](data-formats.md), [`zx-fspack.md`](zx-fspack.md), [`runtime-guide.md`](runtime-guide.md) |
| FSPK module refactor and adapter rename | [`architecture.md`](architecture.md), [`cli.md`](cli.md), [`zx-fspack.md`](zx-fspack.md), [`export-fidelity-contract.md`](export-fidelity-contract.md) |

## Current Rule

Do not keep completed implementation plans under `docs/plans/`. When a new plan
is finished:

1. Move lasting decisions, examples, and compatibility notes into permanent
   docs.
2. Move remaining release blockers into
   [`production-readiness-plan.md`](production-readiness-plan.md).
3. Delete the temporary plan file.
