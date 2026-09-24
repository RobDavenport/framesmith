# framesmith-runtime-wasm

Browser training bindings for the optional FrameSmith runtime helpers, not a
general engine adapter. Build from the repository root with `npm run wasm:build`;
the generated package belongs in `src/lib/wasm` and is not hand-edited.

`TrainingSession` owns validated binary packs and reuses cached views. It rejects
invalid state indices before mutation and prepares both actors before applying
a restored snapshot. Its standing/blocking/reaction choices are preview policy,
not rules imposed by the generic Rust binary reader. It requires compiled
character tables and at most eight resource definitions. Numeric property
lookups understand v2 payloads and the legacy schema layouts.

`tests/e2e/editor-smoke.spec.ts` exercises the actual generated WASM in Chromium,
including snapshot/replay, invalid input, atomic restore and reaction duration.
Editor IPC in these browser tests is mocked; backend filesystem/command tests
are separate evidence and neither is a native installer acceptance test.
