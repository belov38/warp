# Tech Spec: Custom link chips on vertical tab cards

**Issue:** [warpdotdev/warp#16011](https://github.com/warpdotdev/warp/issues/16011)
**Product spec:** [`product.md`](product.md)

All references below are pinned to `e3464f102afddeb48433979bc1d9fee653f500ad` (master, 2026-09-15).

## Context

The feature is a per-pane, persisted piece of metadata (`Vec<PaneLink>`) plus three surfaces on top of it: the Vertical Tabs renderers, a display setting, and local control actions with `warpctrl` subcommands. Each of those has a direct precedent in the codebase, so the plan is mostly "do what the neighbor does".

### Precedent: custom pane name (APP-4114)

The custom pane name is the closest existing feature: pane-scoped, persisted, threaded into Vertical Tabs only, and mutable through local control. Its checklist in [`specs/APP-4114/TECH.md`](../APP-4114/TECH.md) is reused here almost line for line.

- [`app/src/pane_group/pane/mod.rs (690-709) @ e3464f1`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/pane_group/pane/mod.rs#L690-L709) — `PaneConfiguration` with `custom_vertical_tabs_title: Option<String>`.
- [`app/src/pane_group/pane/mod.rs (777-795) @ e3464f1`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/pane_group/pane/mod.rs#L777-L795) — `set_custom_vertical_tabs_title` / `clear_custom_vertical_tabs_title`: trim, no-op on equality, emit `PaneConfigurationEvent::VerticalTabsTitleUpdated`.
- [`app/src/pane_group/pane/mod.rs (874-882) @ e3464f1`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/pane_group/pane/mod.rs#L874-L882) — `PaneConfigurationEvent`.
- [`app/src/pane_group/mod.rs (7880-7890) @ e3464f1`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/pane_group/mod.rs#L7880-L7890) — `attach_pane` maps `TitleUpdated | VerticalTabsTitleUpdated` to `Event::PaneTitleUpdated`, which the workspace turns into a repaint.
- [`app/src/app_state.rs (129-133) @ e3464f1`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/app_state.rs#L129-L133) — `LeafSnapshot { is_focused, custom_vertical_tabs_title, contents }`.
- [`app/src/pane_group/mod.rs (2227-2239) @ e3464f1`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/pane_group/mod.rs#L2227-L2239) — snapshot reads the title from the live `PaneConfiguration`.
- [`app/src/pane_group/mod.rs (2062-2071) @ e3464f1`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/pane_group/mod.rs#L2062-L2071) and [`(2133-2137)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/pane_group/mod.rs#L2133-L2137) — restore applies the title after the pane exists (immediate and deferred AI-document paths).
- [`crates/persistence/src/schema.rs (300-306) @ e3464f1`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/crates/persistence/src/schema.rs#L300-L306) — `pane_leaves` table; [`crates/persistence/src/model.rs (409-413)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/crates/persistence/src/model.rs#L409-L413) `PaneLeaf` and [`(551-555)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/crates/persistence/src/model.rs#L551-L555) `NewPane`; migration `crates/persistence/migrations/*_add_custom_vertical_tabs_title*` (`ALTER TABLE pane_leaves ADD custom_vertical_tabs_title TEXT;`).
- [`app/src/persistence/sqlite.rs:1231 @ e3464f1`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/persistence/sqlite.rs#L1231) write and [`:2428`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/persistence/sqlite.rs#L2428) read; round-trip test [`app/src/persistence/sqlite_tests.rs:486`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/persistence/sqlite_tests.rs#L486).

### Vertical Tabs chips today

- [`app/src/workspace/view/vertical_tabs.rs (5354-5413) @ e3464f1`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L5354-L5413) — `render_terminal_metadata_line`: `Flex::row` with `SpaceBetween`, a `Shrinkable` branch/cwd text on the left, the right badges in a container with 4px left padding, all inside a `ConstrainedBox` of `METADATA_ROW_HEIGHT` ([`:117`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L117)). Only terminal rows call it ([`:4538`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L4538) from `render_terminal_row_content`).
- [`(5415-5458)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L5415-L5458) — `render_terminal_right_badges` reads `vertical_tabs_show_diff_stats` / `vertical_tabs_show_pr_link` and appends chips to a `Flex::row().with_spacing(4.)`.
- [`(5499-5521)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L5499-L5521) — `render_terminal_pull_request_badge`: `Hoverable` with `fg_overlay_1/2` background, `on_click` sends `PrChipClicked` telemetry and dispatches `WorkspaceAction::OpenLink(url)`; [`(5586-5613)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L5586-L5613) `render_badge_container` and `render_pull_request_badge_content` (hard-coded `UiIcon::Github`, 10px label).
- [`(590-593)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L590-L593) — `PaneRowBadgeMouseStates { diff_stats, pull_request }`, one persistent handle per chip, stored per pane in [`pane_badge_mouse_states` / `detail_pane_badge_mouse_states` (703-708)](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L703-L708). Summary mode already grows a `Vec<MouseStateHandle>` on demand for its per-line PR chips at [`(2206-2217)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L2206-L2217).
- [`(822-861)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L822-L861) — `PaneProps`, which already carries `custom_vertical_tabs_title` and `badge_mouse_states`.
- Summary mode: [`VerticalTabsSummaryBranchEntry` (931-939)](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L931-L939), built at [`(3822-3841)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L3822-L3841), coalesced by `(repo_path, branch)` in [`coalesce_summary_branch_entries` (1082-1107)](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L1082-L1107), rendered by [`render_summary_branch_line` (5196-5262)](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L5196-L5262).
- Hover sidecar chips: [`(6884-6913)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L6884-L6913) inside the terminal detail section, reusing the row chip renderers with `VerticalTabsChipEntrypoint::DetailsSidecar`.
- Search text: [`terminal_search_text_fragments` (4127-4145)](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L4127-L4145) and [`summary_search_text_fragments` (1112-1135)](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L1112-L1135) include the PR label.
- Compact rows ([`render_compact_pane_row` :7250](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L7250)) do not render the metadata line, so they get no link chips either (product spec non-goal).
- Telemetry: [`app/src/workspace/view/vertical_tabs/telemetry.rs (14-23, 66-100)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs/telemetry.rs#L14-L100) — `VerticalTabsDisplayOption::ShowPrLink(bool)`, `VerticalTabsChipEntrypoint`, `PrChipClicked { entrypoint }`.
- `WorkspaceAction::OpenLink(String)` ([`action.rs:443`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/action.rs#L443)) is handled by `ctx.open_url(link)` ([`view.rs:24627`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view.rs#L24627)) with no scheme check. That is fine for `gh`-sourced URLs; script-sourced URLs must be validated before they reach the model.

### Display setting

- [`app/src/workspace/tab_settings.rs (545-564) @ e3464f1`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/tab_settings.rs#L545-L564) — `vertical_tabs_show_pr_link` and `vertical_tabs_show_diff_stats` in the settings macro table (bool, default true, `SyncToCloud::Globally`, GUI surface).
- Toggle plumbing: `WorkspaceAction::ToggleVerticalTabsShowPrLink` ([`action.rs:394`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/action.rs#L394), listed as a state-saving action at `:1092`), handler at [`view.rs (25034-25046)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view.rs#L25034-L25046) which flips the setting and sends `DisplayOptionChanged`, repaint hook `TabSettingsChangedEvent::VerticalTabsShowPrLink` at [`view.rs:3896`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view.rs#L3896), popup row via `render_show_toggle_option` at [`vertical_tabs.rs (6115-6131)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/workspace/view/vertical_tabs.rs#L6115-L6131) with per-row mouse states on the panel state (`:727`).

### Local control API

- Catalog: [`crates/local_control/src/catalog.rs (193-217) @ e3464f1`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/crates/local_control/src/catalog.rs#L193-L217) — `tab` and `pane` blocks, including `tab.rename` / `pane.rename` / `pane.reset_name`; [`ActionParameterSpec` (34-55)](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/crates/local_control/src/catalog.rs#L34-L55). `ActionKind::ALL` is generated by the same macro (`:121`).
- Params: [`crates/local_control/src/protocol.rs (143-146)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/crates/local_control/src/protocol.rs#L143-L146) `RenameParams { title }` with `deny_unknown_fields`; resolver mapping at [`app/src/local_control/resolver.rs (31-56)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/local_control/resolver.rs#L31-L56).
- Handlers: [`app/src/local_control/handlers/metadata_config.rs (31-47)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/local_control/handlers/metadata_config.rs#L31-L47) `tab_rename`, [`(108-131)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/local_control/handlers/metadata_config.rs#L108-L131) `pane_rename` / `pane_reset_name`, [`(445-465)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/local_control/handlers/metadata_config.rs#L445-L465) `set_pane_name` (resolves the pane inside `pane_group.update`, mutates `PaneConfiguration`, emits `Event::AppStateChanged` so the change is persisted), [`(211-221)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/local_control/handlers/metadata_config.rs#L211-L221) `rename_title` validation. Dispatch in [`app/src/local_control/bridge.rs (129-158)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/local_control/bridge.rs#L129-L158).
- Pane listing JSON: [`app/src/local_control/handlers/metadata.rs (498-509)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/local_control/handlers/metadata.rs#L498-L509).
- CLI: [`crates/warp_cli/src/local_control/mod.rs (277-315)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/crates/warp_cli/src/local_control/mod.rs#L277-L315) `TabCommand` with nested `TabColorCommand`, [`(319-352)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/crates/warp_cli/src/local_control/mod.rs#L319-L352) `PaneCommand`, [`TargetArgs` (563-605)](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/crates/warp_cli/src/local_control/mod.rs#L563-L605); dispatch in [`commands.rs (400-417, 476-482)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/crates/warp_cli/src/local_control/commands.rs#L400-L417); command-to-action mapping test at [`crates/warp_cli/src/local_control_tests.rs:621`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/crates/warp_cli/src/local_control_tests.rs#L621).
- Session addressing today: the shell bootstrap exports `WARP_SESSION_ID` (numeric `SessionId`, substituted at [`crates/warp_terminal/src/bootstrap.rs:67`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/crates/warp_terminal/src/bootstrap.rs#L67), see `app/assets/bundled/bootstrap/zsh_init_shell.sh:6`). The control API's session id is a different value: [`session_values` (metadata.rs 911-925)](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/local_control/handlers/metadata.rs#L911-L925) reports `pane_id.to_string()` (`Pane Terminal (N)`) as `session_id`, and [`select_session_entries` (50-75)](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/local_control/handlers/metadata.rs#L50-L75) matches `SessionTarget::Id` against that string. Pane and tab mutation handlers reject session selectors outright ([`metadata_config.rs (293-296)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/local_control/handlers/metadata_config.rs#L293-L296)). The bridge from the shell id to a pane exists on the view: [`TerminalView::active_block_session_id` (view.rs 8102-8106)](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/terminal/view.rs#L8102-L8106) returns the `SessionId` of the active block without locking the terminal model, and [`PaneGroup::terminal_view_from_pane_id` (pane_group/mod.rs:7150)](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/app/src/pane_group/mod.rs#L7150) gets there from a `PaneEntry`.
- The control surface spec pins the catalog size: "exactly 84 actions" in [`specs/warp-control-cli/README.md:3`](../warp-control-cli/README.md), `PRODUCT.md:3,11`, `TECH.md:2,25,30,237`, enforced by [`crates/local_control/src/protocol_tests.rs (166-167)`](https://github.com/warpdotdev/warp/blob/e3464f102afddeb48433979bc1d9fee653f500ad/crates/local_control/src/protocol_tests.rs#L166-L167).

## Proposed changes

### 1. Model: `PaneLink` on `PaneConfiguration`

In `app/src/pane_group/pane/mod.rs`:

```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaneLink {
    pub label: String,
    pub url: String,
}

pub const MAX_PANE_LINKS: usize = 3;
pub const MAX_PANE_LINK_LABEL_CHARS: usize = 64;
pub const MAX_PANE_LINK_URL_CHARS: usize = 2048;

pub struct PaneConfiguration {
    // existing fields...
    custom_links: Vec<PaneLink>,
}

impl PaneConfiguration {
    pub fn custom_links(&self) -> &[PaneLink];
    /// Appends or replaces by label. Returns Err(PaneLinkError::TooManyLinks) at the cap.
    pub fn set_custom_link(&mut self, link: PaneLink, ctx: &mut ModelContext<Self>) -> Result<(), PaneLinkError>;
    /// Returns false when no link has that label.
    pub fn remove_custom_link(&mut self, label: &str, ctx: &mut ModelContext<Self>) -> bool;
    pub fn clear_custom_links(&mut self, ctx: &mut ModelContext<Self>);
    /// Used by restore only: replaces the whole list without validation.
    pub fn replace_custom_links(&mut self, links: Vec<PaneLink>, ctx: &mut ModelContext<Self>);
}
```

- `PaneLink::validate(label, url) -> Result<PaneLink, PaneLinkError>` owns invariants 2 and 3 of the product spec: trim, length caps, no control characters, `url::Url::parse` with scheme `http`/`https`. It is a pure function so it can be unit-tested in isolation and shared by the handler.
- Setters follow `set_custom_vertical_tabs_title`: no-op without an event when the resulting list is equal; otherwise emit a new `PaneConfigurationEvent::LinksUpdated`. Do not emit `HeaderContentChanged` (pane headers never show links).
- `attach_pane` (`pane_group/mod.rs:7880`) adds `LinksUpdated` to the match that emits `Event::PaneTitleUpdated`, which is what makes Vertical Tabs repaint.

Storing on `PaneConfiguration` rather than in a `HashMap<PaneId, _>` on `PaneGroup` is what makes invariants 20 and 21 (links follow the pane on move) fall out of the existing pane-move code, exactly as it does for the custom name.

### 2. Snapshot and persistence

- `LeafSnapshot` gains `custom_links: Vec<PaneLink>` (empty when none). `snapshot_for_node` reads it from the live configuration next to the custom title (`pane_group/mod.rs:2227`), and both restore paths (`:2062`, `:2133`) apply it with `replace_custom_links` after the pane exists.
- Migration `crates/persistence/migrations/2026-09-15-000000_add_custom_links_to_pane_leaves/` with `ALTER TABLE pane_leaves ADD custom_links TEXT;` and `down.sql` `ALTER TABLE pane_leaves DROP COLUMN custom_links;`, matching the custom-title migration. Column is nullable; `NULL` and the empty string both read back as no links.
- `schema.rs`: `custom_links -> Nullable<Text>`. `model.rs`: `PaneLeaf` and `NewPane` gain `custom_links: Option<String>`.
- `sqlite.rs`: write `serde_json::to_string(&snapshot.custom_links)` when the list is non-empty, else `None`; read with `serde_json::from_str` and fall back to an empty list on parse failure (log at warn, never fail restore). JSON in one nullable column keeps the schema change to a single line and matches how other list-shaped metadata is stored.
- Update every `LeafSnapshot { .. }` literal in `sqlite_tests.rs` and elsewhere (`custom_links: Vec::new()`).

### 3. Vertical Tabs rendering

In `app/src/workspace/view/vertical_tabs.rs`:

- `PaneRowBadgeMouseStates` gains `links: Vec<MouseStateHandle>`. Add a helper `link_mouse_state(&mut self, index) -> MouseStateHandle` that grows the vector on demand (same pattern as the Summary PR handles at `:2206`). Because the struct is stored per pane in `RefCell<HashMap<PaneId, _>>`, handles stay stable across frames without changing the map's shape.
- `PaneProps` gains `custom_links: Vec<PaneLink>`, filled in `PaneProps::new` from the pane's configuration alongside `custom_vertical_tabs_title`.
- `render_terminal_right_badges` reads the new `vertical_tabs_show_links` setting and, after the PR chip, appends one `render_terminal_link_badge(link, index, entrypoint, mouse_state, appearance)` per link. The badge is `render_badge_container(render_link_badge_content(label, appearance), bg)` inside a `Hoverable`, with `UiIcon::Link` (exists in `crates/warp_core/src/ui/icons.rs:78`) in place of the GitHub icon. `on_click` sends `VerticalTabsTelemetryEvent::LinkChipClicked { entrypoint }` and dispatches `WorkspaceAction::OpenLink(url)`. Display label comes from a pure `link_chip_display_label(&str) -> String` (16 chars + ellipsis, char-aware).
- Because the whole right-badge row lives inside the existing `SpaceBetween` flex with the `Shrinkable` text on the left, invariant 14 (text clips, chips never wrap, height unchanged) needs no new layout code.
- Summary: `VerticalTabsSummaryBranchEntry` gains `links: Vec<PaneLink>`; the builder at `:3822` copies the pane's links, `coalesce_summary_branch_entries` unions them by label (first wins), and `render_summary_branch_line` renders them after the PR chip using the same badge renderer. Mouse handles: extend `summary_pr_badge_mouse_states` from `Vec<MouseStateHandle>` to `Vec<(MouseStateHandle, Vec<MouseStateHandle>)>` per branch line, or add a sibling `summary_link_badge_mouse_states: RefCell<HashMap<PaneId, Vec<Vec<MouseStateHandle>>>>`; the sibling map is less invasive and is the default choice. For a tab with links but no branch entries, `VerticalTabsSummaryData` gains `unattached_links: Vec<PaneLink>` and the summary renderer draws one extra metadata line of chips only (product invariant 16).
- Focused-session mode reuses the pane row, so it needs no change.
- Sidecar: in the terminal detail section (`:6884`) append link chips after the PR chip, using `detail_pane_badge_mouse_states` handles and `VerticalTabsChipEntrypoint::DetailsSidecar`. Labels use the full string.
- Search: `terminal_search_text_fragments` and `summary_search_text_fragments` push each link label.
- Telemetry: add `VerticalTabsDisplayOption::ShowLinks(bool)` and `VerticalTabsTelemetryEvent::LinkChipClicked { entrypoint }` (payload: `entrypoint` only; never the URL or label).

### 4. Setting and popup toggle

- `tab_settings.rs`: add `vertical_tabs_show_links: VerticalTabsShowLinks` with the same shape as `vertical_tabs_show_pr_link` and `toml_path: "appearance.vertical_tabs.show_links"`, description "Whether to show custom link chips on vertical tabs."
- `WorkspaceAction::ToggleVerticalTabsShowLinks`, handled next to `ToggleVerticalTabsShowPrLink` in `view.rs:25034` (flip, send `DisplayOptionChanged(ShowLinks(new))`), added to the state-saving action list at `action.rs:1092`, and to the repaint match at `view.rs:3896` via the generated `TabSettingsChangedEvent::VerticalTabsShowLinks`.
- Popup: a third `render_show_toggle_option("Links", ...)` row after "Diff stats" (`vertical_tabs.rs:6124`), with a new `show_links_mouse_state` on the panel state. No info tooltip.

### 5. Local control actions

- `crates/local_control/src/catalog.rs`: add to the `tab` block
  `TabLinksSet => { name: "tab.links.set", params: LinkSet, result: Acknowledgement }`,
  `TabLinksRemove => { name: "tab.links.remove", params: LinkRemove, ... }`,
  `TabLinksClear => { name: "tab.links.clear", params: None, ... }`,
  and the same three under `pane` as `pane.links.*` with `target: Pane`. All `status: Implemented`.
- `protocol.rs`: `LinkSetParams { label: String, url: String }` and `LinkRemoveParams { label: String }`, both `deny_unknown_fields`; `ActionParameterSpec::{LinkSet, LinkRemove}` and the two `resolver.rs` arms.
- `metadata_config.rs`: `tab_links_set / tab_links_remove / tab_links_clear` select the tab with `select_single_tab_entry`, resolve its focused pane (`pane_group.focused_pane_id`), and delegate to a shared `mutate_pane_links(entry, op, ctx)` that mirrors `set_pane_name`: resolve the pane inside `pane_group.update`, call the model setter, map `PaneLinkError` to `ErrorCode::InvalidParams` with the messages from the product spec, emit `Event::AppStateChanged`, and return the resulting `links` array in the JSON result next to the usual `action` / `tab_id` / `pane_id` fields. `pane_links_*` do the same via `select_single_pane_entry`. Validation runs through `PaneLink::validate` before the model is touched.
- Session addressing (product invariant 8a): a new `select_pane_entry_for_session(target, action, ctx) -> Result<PaneEntry, ControlError>` in `metadata_config.rs` handles `target.session`:
  - `SessionTarget::Active` delegates to `active_session_target` + `select_single_pane_entry` (existing behavior for `session.inspect`).
  - `SessionTarget::Id { id }` parses `id` as a `u64` (`WARP_SESSION_ID`); if it instead matches the `Pane Terminal (N)` form, it is compared against `pane_id.to_string()` like `select_session_entries` does. For the numeric form it walks `pane_entries_for_tabs` (narrowed by the window selector when present), keeps entries whose `terminal_view_from_pane_id(...).active_block_session_id() == Some(SessionId::from(n))`, and requires exactly one match (`missing_target` for zero, `ambiguous_target` for more than one, which should not happen).
  - Any tab or pane selector alongside a session selector is rejected with `invalid_params` before resolution.
  All six link handlers call this helper first when `target.session.is_some()`, and `tab.links.*` then operates on the resolved pane rather than the tab's focused pane. `resolver.rs::validate_action_target` needs no change: `TargetScope::Tab` / `Pane` already allow session selectors at the protocol level; only the handlers rejected them.
- `metadata.rs`: `pane_list` entries gain `"links": [{label, url}, ...]`.
- `bridge.rs`: six dispatch arms.
- Update the control surface docs and test: `specs/warp-control-cli/{README,PRODUCT,TECH}.md` "84" becomes "90", `protocol_tests.rs::catalog_has_exactly_84_retained_actions` becomes 90, and the action table in `PRODUCT.md` lists the six new actions under Tab and Pane.

### 6. `warpctrl`

- `crates/warp_cli/src/local_control/mod.rs`: `TabCommand::Links(#[command(subcommand)] LinksCommand)` and `PaneCommand::Links(LinksCommand)`, with

  ```rust
  pub enum LinksCommand {
      Set(LinkSetArgs),      // --label, --url, flattened TargetArgs
      Remove(LinkRemoveArgs), // --label, flattened TargetArgs
      Clear(TargetArgs),
  }
  ```

  Clap enforces required flags (`--label` non-empty via `value_parser`).
- `commands.rs`: dispatch to `run_action_with_params` with the new params types, modeled on `TabColorCommand`.
- `local_control_tests.rs`: extend the command-to-action mapping test; add parse tests for the three subcommands at both levels.

### Tradeoffs

- **Tab-level and pane-level actions (6) vs pane-level only (3).** Pane-level is the primitive and is all the UI needs; tab-level exists because the issue and every hook author think in tabs, and because `tab.rename` / `pane.rename` already set the precedent of offering both. The extra three actions are thin wrappers over the same function.
- **Reject at the cap vs evict oldest.** Rejecting is deterministic and surfaces misuse in the script; eviction would silently drop a link the user may still want. Scripts that rotate links call `clear` first.
- **JSON column vs child table.** A `pane_links` child table is the "proper" relational shape, but the list is capped at three, is read and written only as a whole, and has no queries against it. One nullable TEXT column keeps the migration and the sqlite read/write code to a handful of lines.

## Testing and validation

Unit tests (`cargo nextest run`):

- New `app/src/pane_group/pane/pane_link_tests.rs` (a `#[path]` test file like `terminal_pane_tests.rs`; inline test modules are rejected by `script/check_no_inline_test_modules`): `PaneLink::validate` accepts/rejects per invariant 2 (trim, 64/2048 limits, control characters, `ftp://`, `javascript:`, relative URL); `set_custom_link` append order, replace-in-place keeps position (4), cap error at four (3), no event on identical set; `remove_custom_link` order preservation and `false` for unknown label (5); `clear_custom_links` idempotent (6).
- `app/src/persistence/sqlite_tests.rs`: round trip of a leaf with three links (order and exact strings) and of a leaf with none; a hand-written invalid JSON value in `custom_links` restores as empty without error (21).
- `app/src/workspace/view/vertical_tabs_tests.rs`: `link_chip_display_label` at 15, 16, 17 chars and with multi-byte characters (13); `coalesce_summary_branch_entries` unions links by label with first-wins (16); search fragments include link labels for row and summary (19).
- `crates/local_control/src/protocol_tests.rs`: `LinkSetParams` / `LinkRemoveParams` serde round trip and `deny_unknown_fields`; catalog count 90; every `*.links.*` action has the expected target scope and parameter spec.
- `app/src/local_control/mod_tests.rs`: handler tests through the bridge for `pane.links.set` (append, replace, cap error message), `remove` (unknown label error), `clear`, `tab.links.set` applying to the focused pane of a split tab (8), stale pane after close (24), result JSON carrying `links` (7), and `pane.list` exposing `links` (9). Session addressing (8a): numeric `--session` resolves the non-focused pane of a split tab whose active block carries that session id; unknown id yields `missing_target`; `--session` plus `--pane` yields `invalid_params`; `tab.links.set --session` targets the session's pane and not the focused one.
- `crates/warp_cli/src/local_control_tests.rs`: subcommand parsing, required-flag errors (10), command-to-action mapping.
- `app/src/workspace/tab_settings_tests.rs`: default and toml path of `vertical_tabs_show_links` (18).

Integration test (`crates/integration`, registered per `INTEGRATION_TESTING.md`): open a terminal tab with Vertical Tabs enabled, set two links on the active pane through the local-control bridge, assert the pane's configuration and that the rendered panel contains the two labels; toggle `vertical_tabs_show_links` off and assert they are gone; toggle on and assert they return. The local-control integration surface does not have a driver in `crates/integration` today, so if wiring the bridge into the test harness turns out to be more than a small addition, this test is limited to setting links on the model directly and the CLI path is covered by the handler tests above plus manual testing.

Manual testing (recorded for the PR, all three item modes plus the sidecar, before/after screenshots):

1. Dev build with Settings > Scripting enabled and Vertical Tabs shown.
2. `warpctrl tab links set --label DELI-1878 --url https://linear.app/...` then a second link; confirm chips, order, hover, and that clicking opens the browser (12, 13, 15).
3. Narrow the sidebar until the branch text disappears; confirm the row height does not change and chips are never wrapped (14).
4. Switch Tab item to Focused session and to Summary; confirm chips in both, including the union behavior with two panes in a split tab (16). Hover the row and confirm chips in the sidecar (17).
5. Toggle Links in the display popup off and on (18). Type a label in the tabs search box (19).
6. Rename the tab, change its color, `cd` to another directory, switch branch, run a Claude Code session to completion; confirm links are unchanged (20).
7. Move the pane to another tab, quit and relaunch Warp; confirm links restore in order (21).
8. Attempt a fourth link, a `javascript:` URL, an empty label, and `remove` of an unknown label; confirm the errors and that nothing changed (2, 3, 5).

## Parallelization

Not proposed. The change is about a dozen files that all hang off the `PaneLink` type introduced in step 1, and the rendering step is where most of the judgment lives; a single implementer on `belov38/tab-link-chips` finishing steps 1 and 2 first, then 3 and 4, then 5 and 6, is faster than coordinating worktrees for roughly two days of work. Steps 5 and 6 (API and CLI) could be handed to a second agent in a separate worktree once step 1 lands, if wall-clock time matters.

## Risks and mitigations

- **Untrusted URLs reaching `OpenLink`.** `ctx.open_url` opens anything the OS will handle. Validation in `PaneLink::validate` is the only gate, so it must run on the API path and the model must not offer an unchecked setter to callers other than restore. Covered by the validate unit tests and the manual `javascript:` case.
- **Metadata row width.** Three chips plus the diff and PR chips can consume the whole row on a narrow sidebar, hiding the branch text. This is the same behavior the PR chip has today at narrower widths, and the `Links` toggle and the cap of three keep it bounded. The product spec's 16-character display truncation is the main lever; if reviewers want a smaller cap or shorter labels, only the two constants change.
- **Catalog size is pinned in three spec files and one test.** Easy to miss one; the presubmit test catches the count, the doc updates are called out in step 5.
- **Old builds reading a new database.** The added column is ignored by older schemas (nullable, additive), and the `down.sql` drops it, matching the custom-title migration.

## Follow-ups

- OSC 777 `warp://cli-agent` notification `links` field so plugins can set chips without `warpctrl` (phase 2 in the issue).
- Unify the two session id forms across the control API: either make every session-scoped action accept the numeric `WARP_SESSION_ID`, or add it to `session.list` output (product spec open question).
- Right-click "Edit links…" or a "Copy link" entry in the pane context menu, and including links in the tab context "Copy metadata" output.
