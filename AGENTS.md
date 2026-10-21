# UI & Copy Guidelines

## No Prompt or Implementation Bleed-Through
- Never expose prompt phrasing, spec terminology, or internal architecture/backend details in user-facing UI text, CLI `--help` output, or `README.md` (e.g., avoid `"Saved Matches (JSON Backend)"`, `"Loose Prompt"`, `"MVP"`, or `"Optional ... after the fact"`).
- Keep all titles, labels, and status messages strictly in the user's domain language.

## Concise `[K]ey` Shortcut Notation
- Use compact inline bracketed-key labels for menus, buttons, and status bars (e.g., `[N]ew match`, `[F]inish`, `[B]oxRec`, `[D]elete`, `[Q]uit`, `[P]ause`) rather than verbose sentences like `"Press [N] to create a new match"`.
- Keep status bar messages brief and avoid duplicating shortcut hints that are already visible in the footer.
- Group action bar shortcuts logically with visual separators (`│`), and hide active-round scoring/timer shortcuts when a fight is already complete.

## Table Alignment, Dates & Single Source of Truth
- **Date Format & Ordering**: Display dates year-first as `YYYY Mon DD` (`%Y %b %d`, e.g., `2024 Oct 16`), and sort match lists in reverse chronological order (most recent first).
- **Fixed-Pivot Column Alignment**:
  - In the `Match` column, right-align Fighter 1 and left-align Fighter 2 around a fixed vertical `v` separator (`{:>max_a} v {:<b}`), keeping clickable BoxRec links aligned to each fighter's name.
  - In the `Scorecard` column, align numeric totals around a fixed `-` separator (`{:>3} - {:<3}`) without repeating fighter names.
- **No Redundant Information**: Do not repeat fighter names or total scores multiple times in the same view. On completed fights, display explicit `WINNER` / `LOSER` / `DRAW` badges inside each fighter's card rather than generic `"FINAL"` labels.

## Documentation & Screenshots (`README.md`)
- Maintain clear getting-started instructions (running via `cargo`, local installation, custom `--file` usage, and running tests).
- Keep keyboard shortcut tables in `README.md` synchronized with the actual TUI keybindings across all screens and modals.
- Whenever UI rendering or version numbers change, regenerate the SVG screenshots in `docs/` via `cargo test -p boxing-tui -- --ignored generate_readme_screenshots`.

# Versioning & Storage Compatibility

## Semantic Versioning (`v0.1.0`+)
- Follow Semantic Versioning (`MAJOR.MINOR.PATCH`) in workspace `Cargo.toml` and git tags (`vX.Y.Z`).
- When cutting a release, ensure `.github/workflows/release.yml` default tag and `Cargo.toml` version are updated together, and that release notes include direct commits and closed issues (`Fixes #N`) between tags.
- Maintain backwards compatibility with `matches.json` files created by `v0.1.0` and all subsequent releases:
  - Always use `#[serde(default)]` (or `#[serde(alias = "...")]`) when adding or evolving fields on serialized structs (`BoxingMatch`, `RoundScore`, `ActiveRound`, `MatchOutcome`, `FinishMethod`).
  - Never remove or rename serialized fields/enum variants without a backwards-compatible serde alias or migration path.
  - Keep `test_v0_1_0_matches_json_backwards_compatibility` in `crates/boxing-core/src/lib.rs` passing at all times, and add new snapshot tests when introducing schema additions in future versions.

## Test Database (`matches.test.json`) Integrity
- Never use placeholder or unverified `boxrec_id` values in `matches.test.json`. Verify fighter BoxRec IDs against authoritative sources (e.g., Wikidata property `P1967` / BoxRec profiles) and keep `test_matches_test_json_integrity` in `crates/boxing-core/src/lib.rs` passing.
