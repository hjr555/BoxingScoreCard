# Boxing Match Scorer

A minimal-input boxing match scoring application built in Rust, designed with a clean hexagonal architecture so any frontend (Console TUI, CLI, MCP server, Web app, or Mobile app) can drive the core scoring engine.

## Workspace Architecture

- **`crates/boxing-core`**: Frontend-agnostic domain library
  - [`BoxingMatch`](crates/boxing-core/src/models.rs): Enforces the `Joshua v Fury (01 Jan 2026)` title format, starts both fighters at `10` points each round, supports point deduction (`-1`), point restoration (`+1` up to `10`), round lock-in, and round undo.
  - [`MatchTimer`](crates/boxing-core/src/timer.rs): Loose countdown timer (default 3m rounds / 1m breaks across 12 rounds) that emits prompts on round starts, 10-second warnings, breaks, and final bell without restricting manual round lock-in.
  - [`MatchRepository`](crates/boxing-core/src/storage.rs) & [`JsonMatchRepository`](crates/boxing-core/src/storage.rs): Pluggable persistence trait with a JSON file backend (`matches.json`) for the MVP.
  - [`ScoringService`](crates/boxing-core/src/service.rs): High-level application service facade ready to be wrapped by a TUI, MCP server, REST/WebSocket API, or mobile FFI.
- **`crates/boxing-tui`**: Full-screen keyboard-driven Console TUI MVP (`ratatui` + `crossterm`).

## Running the Console MVP

```bash
cargo run --bin boxing
```

Optionally specify a custom JSON file path:

```bash
cargo run --bin boxing -- --file my_matches.json
```

## Keyboard Shortcuts (Live Scoring Screen)

| Action | Keyboard Shortcut |
| :--- | :--- |
| **Deduct 1 point** (Fighter 1) | `1`, `Left Arrow`, or `a` |
| **Restore 1 point** (Fighter 1) | `!` (`Shift+1`), `Shift+A`, or `w` |
| **Deduct 1 point** (Fighter 2) | `2`, `Right Arrow`, or `d` |
| **Restore 1 point** (Fighter 2) | `@` (`Shift+2`), `Shift+D`, or `e` |
| **Lock Round In** | `Enter` or `Space` |
| **Undo Last Locked Round** | `u` |
| **Pause / Resume Timer** | `p` |
| **Skip Timer Phase** (Round ↔ Break) | `t` |
| **Return to Matches Menu** | `Esc` or `m` |
| **Quit** | `q` or `Ctrl+C` |
