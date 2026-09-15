# Product Spec: Custom link chips on vertical tab cards

**Issue:** [warpdotdev/warp#16011](https://github.com/warpdotdev/warp/issues/16011)
**Figma:** none provided

## Summary

Let a script or CLI-agent hook attach a small set of named links to a pane, and show them as clickable chips on the pane's card in the Vertical Tabs panel, next to the existing diff-stats and PR chips. Links are set through the local control API (`warpctrl`), stick to the pane across title updates and agent events, survive restart, and can be hidden with a display toggle.

## Problem

The Vertical Tabs card already shows a PR chip, but it only knows about GitHub pull requests discovered through `gh`. Teams on Azure DevOps, GitLab, or with a tracker issue, a CI run, or a preview deployment per tab have no way to put that link on the card. Scripts and agent hooks (for example a Claude Code hook that knows the Linear issue it is working on) have no API to attach a link to the tab they run in.

## Goals

- A script can attach up to three `{label, url}` links to a pane and remove or clear them, through `warpctrl` and the underlying local control actions.
- Link chips render on the pane's card in every Vertical Tabs mode that shows the diff-stats and PR chips, and in the hover details sidecar.
- Clicking a chip opens the URL in the default browser.
- Links persist across app restart and follow the pane when it moves between tabs.
- A `Links` toggle in the Vertical Tabs display settings hides the chips without deleting the links.

## Non-goals

- Editing links from the UI (right-click menu, inline editor). This is a script-facing feature; UI editing can follow if there is demand.
- Rendering link chips on rows that have no metadata line today (Code, Notebook, Settings, and other non-terminal panes). Links can still be set on those panes and are kept, but only terminal rows draw chips.
- Rendering link chips in compact row mode, which does not draw the diff-stats or PR chips either.
- Setting links from launch configs or from the OSC 777 CLI-agent notification. The OSC path is a candidate follow-up once the model and API exist.
- Fetching or validating anything about the URL beyond its syntax (no title lookup, no favicons, no reachability check).

## Behavior

### Data model, from the caller's perspective

1. A link is a `{label, url}` pair. Each pane has an ordered list of zero to three links. Labels are unique within a pane, compared exactly (case-sensitive) after trimming surrounding whitespace.

2. A label is accepted when, after trimming, it is between 1 and 64 characters long and contains no control characters (including newlines). A URL is accepted when, after trimming, it contains no control characters and parses as an absolute URL with scheme `http` or `https`; it is stored and returned in its normalized form (for example `https://a` becomes `https://a/`), and that normalized form is at most 2048 characters long. Any other label or URL is rejected with an `invalid_params` error that names the offending field; the pane's links are left unchanged.

3. `set` with a label that is not yet on the pane appends the link at the end of the list. If the pane already has three links, `set` with a new label fails with `invalid_params` ("pane already has 3 links") and changes nothing. Callers that want to rotate links use `remove` or `clear` first.

4. `set` with a label that already exists on the pane replaces that link's URL in place. The link keeps its position in the list. Setting the same label and URL again is a no-op that still succeeds.

5. `remove` with a label that exists deletes that link; remaining links keep their relative order. `remove` with a label that does not exist fails with `invalid_params` ("no link with label ...") and changes nothing.

6. `clear` removes all links from the pane and succeeds even when the pane has no links.

7. Every successful `set`, `remove`, and `clear` returns the pane's full link list after the mutation, in order, so the caller can confirm the resulting state without a second call.

### Addressing

8. The mutations exist at two levels of the local control catalog, mirroring `tab.rename` / `pane.rename`:
   - `pane.links.set`, `pane.links.remove`, `pane.links.clear` target one pane (default: the active pane of the active tab). They accept the same window, tab, and pane selectors as `pane.rename`.
   - `tab.links.set`, `tab.links.remove`, `tab.links.clear` target one tab (default: the active tab) and apply the mutation to that tab's focused pane. They accept the same selectors as `tab.rename`.

8a. Unlike `pane.rename`, all six actions also accept a session selector, so a script running inside a pane can address that pane without knowing whether it is focused: `--session "$WARP_SESSION_ID"` (the numeric id Warp exports into every bootstrapped shell). The action resolves the terminal pane whose current shell session has that id, across all windows and tabs, and applies the mutation to it. For `tab.links.*` the target is that same pane, not the tab's focused pane.
   - `--session active` resolves to the active pane of the active tab, as it does for `session.inspect`.
   - When no pane currently has that session (the shell exited, or the id is from another Warp instance), the action fails with `missing_target` and changes nothing.
   - A session selector combined with a `--pane`, `--pane-index`, `--tab`, `--tab-index`, or `--tab-title` selector fails with `invalid_params`; window selectors are allowed and narrow the search.

9. `pane.list` and `pane.inspect` include the pane's links as a `links` array of `{label, url}` objects, empty when the pane has none.

10. `warpctrl` exposes the actions as:

    ```
    warpctrl tab links set --label <label> --url <url> [target selectors]
    warpctrl tab links remove --label <label> [target selectors]
    warpctrl tab links clear [target selectors]
    warpctrl pane links set --label <label> --url <url> [target selectors]
    warpctrl pane links remove --label <label> [target selectors]
    warpctrl pane links clear [target selectors]
    ```

    Output follows the existing `warpctrl` conventions (human-readable by default, `--output json` for the structured result). `--label` is required for `set` and `remove`; `--url` is required for `set`. Missing or empty flags are reported by the CLI before any request is sent.

11. As with every local control action, the commands fail with the existing "scripting disabled" error when Settings > Scripting is off, and with the existing "no running Warp" error when no instance is discoverable.

### Rendering on the card

12. On a terminal pane row in the Vertical Tabs panel, link chips render in the metadata line (the line that shows the git branch or working directory on the left and the diff-stats and PR chips on the right). Link chips sit at the right end of that line, after the diff-stats chip and after the PR chip when those are shown, in the pane's link order.

13. A link chip looks like the PR chip: the same pill background, corner radius, padding, and hover highlight, with a generic link icon in place of the GitHub icon and the label as text. The label displays at most 16 characters; longer labels are truncated with an ellipsis. The full label is never modified in storage.

14. The metadata line keeps its fixed height. When the sidebar is too narrow for the branch text plus all chips, the branch (or working directory) text shrinks and clips first, exactly as it does today for the diff-stats and PR chips; once the branch cell is narrower than its icon it is hidden entirely rather than drawn under the chips. Chips are never wrapped onto a second line; chips that still do not fit are clipped at the card edge.

15. Hovering a chip highlights it and shows the pointing-hand cursor. Clicking a chip opens the link's URL in the system browser via the same path the PR chip uses. The click does not focus or activate the pane and does not open any Warp panel.

16. Chips appear in all three Vertical Tabs item modes that show the metadata line:
    - Panes: on each pane's own row.
    - Focused session (Tabs granularity): on the tab's row, showing the focused pane's links.
    - Summary: on the tab's summary card, each branch line shows the union of the links of the terminal panes on that repository and branch, with duplicates by label removed (first occurrence wins, in pane order). A tab with no branch lines at all but with links on its terminal panes draws one metadata line containing only the link chips. Links on branchless panes are not shown when the tab also has branch lines, and non-terminal panes contribute no links (see Non-goals). A branch line displays at most three link chips; any further links in the union remain searchable but are not drawn.

17. The hover details sidecar for a terminal pane shows the pane's link chips in its metadata row, after the diff-stats and PR chips, with the same click behavior. Labels are not truncated in the sidecar.

18. Link chips are hidden when the new `Links` toggle in the Vertical Tabs display settings popup is off. The toggle sits next to the `PR link` and `Diff stats` toggles, defaults to on, and is stored as `appearance.vertical_tabs.show_links`. Turning it off hides chips everywhere (rows, Summary, sidecar) but does not delete links; turning it back on shows them again.

19. Link labels participate in Vertical Tabs search the same way the PR chip label does: typing a label (or part of it) in the tabs search field matches the row.

### Lifetime

20. Links belong to the pane, not to its title or its session state. They are unaffected by shell title changes, working-directory changes, git branch changes, CLI-agent status changes, tab rename, tab color, and pane rename.

21. Links survive app restart and are restored onto the same pane with the same order. They survive a pane move between tabs and a tab move between windows. Closing the pane discards its links.

22. Links are never synced to the cloud and never leave the local machine except when the user clicks a chip.

### Concurrency and races

23. Two scripts mutating the same pane's links are applied in the order the app receives the requests; each request sees the state left by the previous one. There is no batching or last-writer-wins merging across requests.

24. If the target pane is closed between selection and mutation, the action fails with the existing `stale_target` error, as `pane.rename` does.

## Open questions

- **Two meanings of "session id".** Existing actions (`session.list`, `session.inspect`) report and accept a session id that is really the terminal pane's view id (rendered as `Pane Terminal (N)`), while the shell only knows the numeric `WARP_SESSION_ID`. Invariant 8a accepts the numeric form for the six link actions because that is the only id a hook has. Should the other session-scoped actions accept it too, or should `session.list` start reporting the shell id next to the pane id? Either is a small follow-up; this spec only needs the link actions to accept the shell id.
- **Non-terminal rows.** Should Code, Notebook and agent-conversation rows grow a metadata line so links can show there too? Out of scope here; raised so reviewers can steer.
