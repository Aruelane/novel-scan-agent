# novel-core

`novel-core` is the platform-neutral domain and orchestration layer for the
novel scanning agent. It deliberately contains no filesystem, HTTP, Tauri,
Windows, or Android APIs. Those concerns are adapters around this crate.

The first vertical slice provides:

- serializable tasks, rule selections, chapters, findings, and source locators;
- an asynchronous, provider-neutral model interface;
- a deterministic pattern provider for tests and offline development;
- a pluggable context compressor;
- resumable per-chapter checkpoints with a persistence interface;
- exact evidence reconstruction from imported source text.

## Rule classification and alert strength

Rule classification and user alert strength are intentionally separate:

- `RuleCategory::Landmine` / `RuleCategory::Frustration` records the
  community-defined category (`landmine` / `frustration` in JSON).
- `AlertLevel::{Critical, High, Medium, Low, Info}` records the user's chosen
  alert strength. `AlertLevel::from_ui_scale` and `ui_scale` map these values
  to the front-end 5 / 4 / 3 / 2 / 1 settings.

`RuleDefinition`, `RuleSelection`, provider `RuleContext`, and `Finding` all
carry both values. A selection also snapshots the category and is rejected if
it no longer matches the rule catalog. Both values are included in the scan
profile fingerprint, so an in-progress checkpoint cannot resume under a
silently changed rule category or user alert level.

## Finding evidence states

A finding is only `confirmed` when every evidence range can be sliced from the
original UTF-8 chapter text. Model-supplied prose is never treated as source
evidence. The persisted states have distinct meanings:

- `suspected`: evidence is absent or cannot yet be validated against source;
- `pending_confirmation`: an exact clue exists, but later chapters or
  relationship facts are still required;
- `confirmed`: exact source evidence is valid and no later confirmation is
  required;
- `rejected`: a persisted finding has been ruled out by later context or human
  review.

The scanner can emit the first three states. `rejected` remains serializable
for persistence and downstream review workflows. Pending and suspected IDs
remain in compressed context as unresolved candidates. Checkpoint schema v2
contains the separated category and alert-level fields; v1 checkpoints must be
restarted or migrated by the application layer.
