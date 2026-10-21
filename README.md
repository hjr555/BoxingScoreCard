# Boxing Scorecard

A terminal-based boxing scorecard for tracking round-by-round fight scores, pre-fight walkouts, round timers, and fight outcomes.

## Features

- **10-Point Must Scoring**: Starts both fighters at 10 points each round with single-keystroke point deductions (`-1`), restorations (`+1`), round lock-in, and multi-step undo.
- **Pre-Fight Walkout**: Score the ring walks before Round 1 (or revisit anytime with `0`) without affecting the official fight total.
- **Round & Break Timer**: Tracks 3-minute rounds, 1-minute breaks, and 10-second warnings without blocking manual scoring.
- **Stoppages & Decisions**: Record `TKO`, `KO`, `RTD`, `DQ`, `TD`, or full-distance `DEC` outcomes at any point.
- **BoxRec IDs**: Attach Fighter BoxRec IDs to any match.

---

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (edition 2021, 1.75+)

### Run Directly with Cargo

From the repository root:

```bash
cargo run --release --bin boxing
```

By default, matches are saved to `matches.json` in the current working directory. To use a custom file path:

```bash
cargo run --release --bin boxing -- --file path/to/matches.json
```

### Install Locally

To install the `boxing` binary into `~/.cargo/bin`:

```bash
cargo install --path crates/boxing-tui
```

Then run from anywhere:

```bash
boxing
# or with a custom storage file:
boxing --file ~/fights/matches.json
```

### Run Tests

```bash
cargo test
```

---

## Quick Start: Scoring a Fight

1. **Create a Match**: Press `n` on the **Matches** screen, enter the two fighter names, date (e.g. `01 Jan 2026` or `2026-01-01`), and scheduled rounds (`←` / `→`), then press `Enter`.
2. **Score the Walkout**: Deduct points with `1` (Fighter 1) or `2` (Fighter 2), then press `Enter` to lock the walkout and start Round 1.
3. **Score Each Round**: Both fighters start each round at `10`. Press `1` or `2` to deduct points (`9`, `8`, etc.) and `Enter` or `Space` to lock the round and advance.
4. **Finish the Fight**: Locking the final scheduled round automatically computes a decision from your scorecard. If the fight ends early, press `f` (`TKO`), `k` (`KO`), or `r` (`RTD`) to record the stoppage.

---

## Keyboard Shortcuts

### Matches Screen

| Action | Shortcut |
| :--- | :--- |
| **New Match** | `n` |
| **Select Match** | `↑` / `↓` or `k` / `j` |
| **Open Match** | `o` or `Enter` |
| **Finish / Edit Result** | `f`, `s`, or `x` |
| **Edit BoxRec IDs** | `b` |
| **Delete Match** | `d` or `Delete` (`y` / `d` to confirm, `Esc` to cancel) |
| **Quit** | `q` or `Ctrl+C` |

### Scoring Screen

| Action | Shortcut |
| :--- | :--- |
| **Deduct 1 pt (Fighter 1)** | `1`, `←`, or `a` |
| **Restore 1 pt (Fighter 1)** | `!`, `Shift+A`, or `w` |
| **Deduct 1 pt (Fighter 2)** | `2`, `→`, or `d` |
| **Restore 1 pt (Fighter 2)** | `@`, `Shift+D`, or `e` |
| **Lock Walkout / Round** | `Enter` or `Space` |
| **Finish Match (TKO / KO / RTD)** | `f` (TKO), `k` (KO), or `r` (RTD) |
| **Edit Walkout Score** | `0` |
| **Edit BoxRec IDs** | `b` |
| **Undo Last Round / Result** | `u` |
| **Pause / Resume Timer** | `p` |
| **Next Timer Phase** | `t` |
| **Back to Matches** | `Esc` or `m` |
| **Quit** | `q` or `Ctrl+C` |

### Finish Match Modal

| Action | Shortcut |
| :--- | :--- |
| **Select Method** | `t` (TKO), `k` (KO), `r` (RTD), `q` (DQ), `d` (DEC / TD), or `←` / `→` |
| **Select Winner** | `1` (Fighter 1), `2` (Fighter 2), `0` (Draw), or `←` / `→` |
| **Adjust Round** | `-` / `[` (Earlier), `+` / `]` (Later), or `←` / `→` |
| **Switch Field** | `Tab`, `↑`, or `↓` |
| **Confirm Result** | `Enter` or `Space` |
| **Cancel** | `Esc` |

---

## Project Structure

- **`crates/boxing-core`**: Core domain library
  - [`BoxingMatch`](crates/boxing-core/src/models.rs): Match state, walkout and round scoring, outcomes (`TKO`, `KO`, `RTD`, `DQ`, `TD`, `DEC`), and undo logic.
  - [`MatchTimer`](crates/boxing-core/src/timer.rs): Round and break countdown timer with 10-second alerts.
  - [`MatchRepository`](crates/boxing-core/src/storage.rs) & [`JsonMatchRepository`](crates/boxing-core/src/storage.rs): Persistence trait and atomic JSON file storage.
  - [`ScoringService`](crates/boxing-core/src/service.rs): Application service coordinating match operations and storage.
- **`crates/boxing-tui`**: Terminal user interface (`ratatui` + `crossterm`).
