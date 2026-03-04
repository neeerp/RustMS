---
title: "NX-authoritative field data + portal map transfer"
date: 2026-03-04
status: "draft"
owners: ["server", "runtime", "net"]
related:
  - "other_servers/thoughts/shared/research/2026-03-02-map-representation-in-maplestory-servers.md"
  - "other_servers/thoughts/shared/research/2026-03-02-nx-wz-server-data.md"
---

# Goal

Introduce server awareness of MapleStory field/map data, using client `.nx` files as authority, to support portal-based map transitions (`CHANGEMAP` with `target == -1`).

# Decisions locked

- Authoritative game data source: NX (`Map.nx`) via `nx-pkg4`.
- Validation strategy first: compare NX-derived portal data against Cosmic XML data.
- Mismatch severity:
  - **Hard**: map/portal existence (for explicit target maps), `tm`, `tn`
  - **Soft**: `x`, `y`, `pt`, `script`
- Validator must not hardcode `../other_servers`; paths are CLI inputs.
- No large `.nx` fixtures committed to RustMS.
- Local large inputs should live in git-ignored `assets/game-data/`.

# Milestones

## M1: Portal parity validator (first implementation phase)

Create a standalone tool in workspace:

- Crate: `tools/map-data-validator` (binary)
- CLI inputs:
  - `--nx-map <path>`
  - `--cosmic-map-root <path>`
- Optional:
  - `--maps <csv>`
  - `--strict`
  - `--report <path>`

### M1.1 Data model

Normalize both sources into shared records:

- `MapRecord { map_id, portals }`
- `PortalRecord { map_id, id, pn, pt, tm, tn, x, y, script }`

Normalization rules:

- `tn`: empty string and missing treated equivalently.
- `tm == 999999999`: sentinel/no external destination (still represented consistently).
- Preserve portal `id` for primary matching and use `pn` as diagnostic aid.

### M1.2 Readers

- `NxPortalReader` (from `Map.nx` using `nx-pkg4`)
- `CosmicXmlPortalReader` (from `Map/Map*/<id>.img.xml`)

### M1.3 Comparator

Per map:

- detect missing maps/portals
- compare hard fields (`tm`, `tn`)
- compare soft fields (`x`, `y`, `pt`, `script`)

Behavior:

- with `--maps`: missing map/portal is hard failure
- without `--maps`: missing maps are warnings (coverage drift signal)

### M1.4 Output and exit status

- Console summary:
  - maps compared
  - exact matches
  - maps with hard mismatches
  - maps with soft mismatches
- JSON report with detailed diffs and severity
- Exit non-zero on hard mismatches (or strict mode promotion)

### M1.5 Tests (in-repo, small fixtures)

- Unit tests for normalization and severity classification
- Small Cosmic XML fixtures + synthetic normalized records
- No committed NX binaries required
- Optional ignored/manual test path for real NX file

## M2: `game-data` crate (NX field templates)

After validator confidence:

- New crate (working name): `game-data`
- Implement:
  - `FieldTemplate { map_id, return_map, forced_return, portals }`
  - `PortalTemplate { id, name, type, to_map, to_name, x, y, script }`
- APIs consumed by runtime/net:
  - `field_exists(map_id)`
  - `portal_by_name(map_id, portal_name)`
  - `spawn_portal(map_id, preferred_to_name)`

## M3: Net handler portal resolution (`target == -1`)

Update `ChangeMapHandler`:

- Parse existing change-map packet fields
- For `target == -1`:
  - resolve source portal by name in current map
  - destination from `tm`
  - destination spawn portal by `tn` fallback chain
- Persist character `map_id`
- Build and send field/set-map packet(s)
- Emit a map-transfer action/event for runtime field-aware transition

Keep direct-map path (`target != -1`) unchanged initially.

## M4: Runtime field-aware transfer orchestration

Replace current placeholder map-change behavior in world actor:

- Leave old `FieldActor`
- Join/create destination `FieldActor`
- Update registry `field_key`
- Ensure field-local broadcasts remain correct across transfer

## M5: Integration coverage

Add tests to verify:

- valid portal transfer (single player)
- invalid/missing portal handling (no crash, safe response)
- observer behavior when player leaves source map

# Suggested execution order

1. Scaffold `tools/map-data-validator`
2. Implement readers + normalization
3. Implement comparator + report
4. Add tests + docs for manual invocation
5. Run validator on starter maps:
   - `100000000`, `100000001`, `104000000`
6. Expand scan and triage diffs
7. Build `game-data` crate using validated parsing
8. Wire `ChangeMapHandler` + runtime transfer
9. Add integration tests

# Detailed implementation blueprint

## Phase A: Validator scaffolding in `tools/map-data-validator`

Deliverables:

- Add workspace member: `tools/map-data-validator`
- Add CLI argument parsing and help text
- Add report writer and process exit contract

Proposed internal module layout:

- `src/main.rs` - argument parsing, orchestration, summary output
- `src/model.rs` - normalized records and mismatch enums
- `src/nx_reader.rs` - `Map.nx` portal extraction
- `src/cosmic_reader.rs` - Cosmic XML portal extraction
- `src/compare.rs` - comparator and severity classification
- `src/report.rs` - JSON report shape + serialization

Definition of done:

- `cargo run -p map-data-validator -- --help` shows required flags and examples
- Binary exits with clear non-zero code only for hard mismatches

## Phase B: Data extraction and normalization

Deliverables:

- Parse NX map nodes into `MapRecord`/`PortalRecord`
- Parse Cosmic XML map files into same normalized model
- Normalize sentinel/default values (`tn`, `tm`)

Normalization details:

- `tn`: missing and empty become empty string
- `script`: missing and empty become `None`
- `tm`: keep integer value; annotate sentinel (`999999999`) in diagnostics

Definition of done:

- Unit tests cover normalization edge cases
- Sample map pair from both sources yields deterministic normalized outputs

## Phase C: Comparison engine and report schema

Deliverables:

- Per-map parity result with:
  - exact count
  - hard mismatch list
  - soft mismatch list
  - missing-map warnings/errors based on mode
- Global summary totals and failing-condition boolean
- JSON report written to disk

Report schema (high-level):

- `summary { maps_compared, exact_maps, maps_with_hard, maps_with_soft, hard_count, soft_count }`
- `maps[] { map_id, status, hard[], soft[], notes[] }`

Definition of done:

- Human summary and JSON report agree on totals
- `--strict` promotes soft mismatches to hard

## Phase D: Test fixtures and CI-safe coverage

Deliverables:

- Small XML fixtures in repo
- Synthetic normalized fixture objects for comparator tests
- Optional ignored test for local real-data execution

Definition of done:

- `cargo test -p map-data-validator` passes without local external data
- Manual run instructions documented for real NX/Cosmic paths

## Phase E: `game-data` crate extraction from validated reader logic

Deliverables:

- New crate: `game-data`
- Move/reuse stable parsing pieces from validator into library API
- Runtime-facing lookup API for fields and portals

Proposed API surface:

- `GameData::load_from_nx(map_nx_path)`
- `GameData::field(map_id) -> Option<&FieldTemplate>`
- `FieldTemplate::portal_by_name(name)`
- `FieldTemplate::resolve_spawn_portal(preferred_to_name)`

Definition of done:

- Unit tests for lookup and fallback behavior
- No runtime crate depends on validator binary modules

## Phase F: Net/runtime integration for portal transfers

Deliverables:

- `ChangeMapHandler` uses `game-data` when `target == -1`
- Add handler action variant for explicit map transfer intent
- Runtime world actor performs field-aware transfer (`Leave -> Join`)

Expected file touchpoints:

- `net/src/packet/handle/world/change_map.rs`
- `net/src/handler.rs` (action enums/result plumbing)
- `runtime/src/message.rs` (event enum updates)
- `runtime/src/actor/client.rs` (action->event mapping)
- `runtime/src/actor/world.rs` (field-aware transfer orchestration)

Definition of done:

- Player portal request changes map based on NX portal linkage
- Field broadcasts route to new field after transition

## Phase G: End-to-end validation

Deliverables:

- Integration harness test for successful portal transfer
- Integration harness test for invalid portal handling
- Optional observer test for visibility in source map

Definition of done:

- Tests pass consistently and guard against regression in transfer behavior

# Open design points (to resolve during implementation)

- Where to host `GameData` singleton lifecycle (world startup vs lazy static)
- Whether to include destination portal coordinates in first transfer iteration
- How to represent and surface script-driven portals not handled yet
- Whether to gate portal usage by portal `pt` type in first implementation

# Risks and mitigations

- NX path/node interpretation drift:
  - Mitigate with validator before runtime integration.
- Dataset/version mismatch NX vs Cosmic:
  - Track hard/soft diffs and target-map allowlist.
- Over-coupling parser to one tool:
  - Keep normalized records + loader interfaces clean and reusable.

# Done criteria for this plan

- M1 done when validator works end-to-end, has tests, and reports hard/soft diffs reliably.
- M2-M5 done when portal transitions through map data work in runtime and are covered by integration tests.
