# UI & Copy Guidelines

## No Prompt or Implementation Bleed-Through
- Never expose prompt phrasing, spec terminology, or internal architecture/backend details in user-facing UI text (e.g., avoid `"Saved Matches (JSON Backend)"`, `"Loose Prompt"`, `"MVP"`, or `"Optional ... after the fact"`).
- Keep all titles, labels, and status messages strictly in the user's domain language.

## Concise `[K]ey` Shortcut Notation
- Use compact inline bracketed-key labels for menus, buttons, and status bars (e.g., `[N]ew match`, `[F]inish`, `[B]oxRec`, `[D]elete`, `[Q]uit`, `[P]ause`) rather than verbose sentences like `"Press [N] to create a new match"`.
- Keep status bar messages brief and avoid duplicating shortcut hints that are already visible in the footer.

# Versioning & Storage Compatibility

## Semantic Versioning (`v0.1.0`+)
- Follow Semantic Versioning (`MAJOR.MINOR.PATCH`) in workspace `Cargo.toml` and git tags (`vX.Y.Z`).
- Maintain backwards compatibility with `matches.json` files created by `v0.1.0` and all subsequent releases:
  - Always use `#[serde(default)]` (or `#[serde(alias = "...")]`) when adding or evolving fields on serialized structs (`BoxingMatch`, `RoundScore`, `ActiveRound`, `MatchOutcome`, `FinishMethod`).
  - Never remove or rename serialized fields/enum variants without a backwards-compatible serde alias or migration path.
  - Keep `test_v0_1_0_matches_json_backwards_compatibility` in `crates/boxing-core/src/lib.rs` passing at all times.
