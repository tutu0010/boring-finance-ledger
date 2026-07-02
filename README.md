# Personal Finance Ledger

A lean, event-sourced personal finance ledger written in Rust.

Track money intentionally. Learn Rust deeply. Own your data.

---

## Why This Project Exists

Personal Finance Ledger started as a learning project. I wanted to learn Rust by building something I would genuinely use every day rather than creating another tutorial application, toy project, or CRUD example that would be abandoned after a weekend.

At the same time, I needed a better way to track my university finances. Most finance applications optimize for convenience, automation, and reducing user interaction as much as possible. While that approach works well for many people, I found that manually recording expenses made me far more aware of where my money was going.

This project embraces what I call **Intentional Friction**. Instead of hiding financial decisions behind automatic categorization, bank integrations, and background synchronization of *transactions*, every entry is still typed deliberately at a terminal. The terminal is not a limitation; it is part of the experience.

The result is a lightweight, primarily local finance ledger that helps me learn Rust while remaining genuinely useful in daily life.

---

## Why You Might Like It

This project is not trying to compete with commercial finance software. Instead, it targets a very specific type of user.

If you enjoy terminal workflows, want a deeper understanding of your spending habits, or are learning Rust through practical projects, this ledger may be exactly what you're looking for. Every transaction is entered by hand, on purpose. There's no bank integration, no automatic categorization, no ads, no subscriptions.

Most importantly, it is small enough to understand completely. Every major design decision can be traced back to a simple principle: keep the system understandable.

---

## The Core Idea: Events, Not State

Most expense trackers store the current state of your finances directly.

For example:

```text
alex owes me ₹300
```

This project does not store information like that.

Instead, it stores financial events:

```text
LoanGiven ₹500 to alex
RepaymentReceived ₹200 from alex
```

The current balance is calculated from those events whenever it is needed.

This approach is known as **Event Sourcing**, and it provides several benefits. Every financial action is preserved permanently, historical reports become easy to generate, balances can always be recomputed, and the ledger never has to duplicate information.

Even correcting a mistake follows this rule. Running `undo` doesn't delete the event you're correcting — it appends a new `Reversed` event that points back at it. The original stays in the log, just excluded from every balance and summary calculation from that point on. Nothing is ever erased; the ledger only ever grows.

One rule drives the entire architecture:

```text
Commands create Events.
Queries read Events.
```

If something creates a new financial fact, it becomes an event. If something only reads existing information, it becomes a query. As of V3, this split exists at the type level, not just as a convention — a query can never accidentally be routed to the code path that writes to the ledger, because `Command` and `Query` are two separate Rust types with no overlap.

---

## Architecture

```text
CLI
 │
 ▼
Parser (Action)
 │
 ▼
Dispatch  ──── splits into ────►  Command (writes)      Query (reads)
                                       │                      │
                                       ▼                      ▼
                                    Ledger                query::execute
                                       │
                                       ▼
                                     Event
                                       │
                                       ▼
                                  EventStore (trait)
                                       │
                                       ▼
                          Encrypted, append-only local log
                             + optional Supabase push
```

`Command` and `Query` are distinct types. `Ledger` only ever accepts a `Command` — it is structurally impossible to hand it something read-only. Everything the ledger knows is still derived from the event log; there is no separate balance table, debt table, or summary table anywhere in the system.

Storage itself sits behind an `EventStore` trait. `Ledger` doesn't know or care whether events are landing in a local file, a database, or anywhere else — it just calls `load()` and `append()`. That's what makes the sync layer below possible without touching a single line of the ledger's core logic.

---

## Storage and Security

The ledger stores data locally as an append-only, line-encrypted log (`ledger.jsonl`). Each event is encrypted independently with its own random nonce (XChaCha20-Poly1305), so appending a new entry never touches or re-encrypts anything already written — the file only ever grows, and a crash mid-write can corrupt at most the very last line, never anything before it.

Your passphrase never touches disk. It's used once, at startup, to derive an encryption key via Argon2id, and the derived key is zeroed out of memory the moment the process exits. A small `ledger.meta.json` file stores a random salt and a "canary" — a known value encrypted under your key — so a wrong passphrase is rejected immediately with a clear error, instead of surfacing forty commands later as unreadable garbage.

If you're upgrading from an earlier plaintext version of this ledger, the first run automatically migrates your old `ledger.json` into the new encrypted format and renames the original to `ledger.json.migrated` rather than deleting it.

A backup snapshot (`ledger.jsonl.bak`) refreshes periodically as a recovery point, on top of the append-only design already making the log hard to corrupt in the first place.

Financial amounts are never a bare `Decimal` — they're wrapped in a `Money` type that rejects negative or zero values and rounds to two decimal places at the moment of construction. There's no code path anywhere in the ledger where an amount can be invalid; it simply can't be built.

---

## Optional Cloud Sync

Personal Finance Ledger is not a cloud product, and it doesn't require an internet connection to work. But if you set two environment variables — `SUPABASE_URL` and `SUPABASE_KEY` — every command will also push new events to a Supabase Postgres table after saving them locally.

This is entirely opt-in. Leave those variables unset and the ledger behaves exactly as before: fully local, fully offline. Set them, and you get a second, queryable copy of your ledger in the cloud — useful as a backup, or as a foundation for a future read-only mobile view.

Sync is resilient by design: if Supabase is unreachable, the command you actually ran still succeeds using the local encrypted log, and any events that didn't sync are simply retried on the next run. The local store is always the source of truth; the cloud copy is a mirror, never a dependency.

---

## Features

The ledger currently supports recording income, expenses, loans given to other people, loans taken from other people, repayments, and recurring subscriptions. Each of these actions becomes an immutable event in the ledger.

Subscriptions actually charge now. On every run, the ledger checks how many billing periods have elapsed since a subscription's last charge and materializes the missing `Expense`-equivalent events automatically — a `subscribe` command isn't just a note that a subscription exists, it's a running record of what it's actually cost you.

Category and person names are normalized on entry (`food` and `Food` land in the same bucket), so typo-driven double-counting in summaries and balances isn't something you have to manage by hand.

Queries allow you to view event history, generate summaries, inspect outstanding debts, review money owed to you, search the ledger, and filter information by category or event type.

All balances and reports are derived directly from the event history rather than being stored separately.

---

## Recording Financial Events

The following commands create new events inside the ledger.

| Action            | Example                                     |
| ----------------- | ------------------------------------------- |
| Expense           | `ft expense 200 food burger_king`           |
| Income            | `ft income 15000 father semester_fees`      |
| Lend Money        | `ft lend 500 alex lunch`                    |
| Borrow Money      | `ft borrow 1000 max emergency_cash`         |
| Receive Repayment | `ft receive 200 alex`                       |
| Repay Debt        | `ft repay 400 max`                          |
| Subscription      | `ft subscribe 89.9 spotify monthly`         |
| Undo Last Event   | `ft undo`                                   |

Every command immediately creates a new event and persists it to storage. `undo` reverses the most recent event by appending a compensating `Reversed` event — see [The Core Idea](#the-core-idea-events-not-state) above.

---

## Querying the Ledger

Queries never modify data. They simply read information derived from the event history.

| Query                            | Description                   |
| --------------------------------- | ----------------------------- |
| `ft history`                      | Show every recorded event     |
| `ft summary`                      | Display financial overview    |
| `ft summary 2026-01 2026-12`      | Summary for a date range      |
| `ft owed`                         | Show people who owe you money |
| `ft debts`                        | Show money you owe others     |
| `ft list expenses`                | List expense events           |
| `ft list income`                  | List income events            |
| `ft list loans`                   | List loan-related events      |
| `ft list subscriptions`           | List subscriptions            |
| `ft find alex`                    | Search for a person           |
| `ft find food`                    | Search by category            |
| `ft find burger`                  | Search by merchant            |
| `ft find spotify`                 | Search by subscription        |

The `summary` command is intended to be the flagship query. It provides an overview of income, expenses, savings, category breakdowns, subscriptions, and debt statistics.

---

## Example Session

A typical session might look like this:

```bash
ft income 15000 father semester_fees

ft expense 120 food burger_king

ft lend 500 alex lunch

ft receive 200 alex

ft summary
```

Which could produce something similar to:

```text
--- Summary ---
Income:   ₹15000.00
Expenses: ₹120.00
Net:      ₹14880.00

Category Breakdown:
  Food: ₹120.00

Debt Statistics:
  Owed to you: ₹300.00
  You owe:     ₹0.00
```

The balance owed by alex is never stored directly. It is calculated from the recorded loan and repayment events.

---

## Technical Decisions

Several design decisions were made intentionally.

Event sourcing was chosen because it preserves complete financial history while avoiding duplicated state. Decimal arithmetic was chosen because money should never be represented using floating-point values — and as of V3, that decimal is wrapped in a `Money` type so an invalid amount can't be constructed in the first place, not just avoided by convention.

Storage moved from plain JSON to an encrypted, append-only log because this is a *finance* ledger — the convenience of `cat`-ing a plaintext file directly wasn't worth every category, person, and amount being readable by anything with filesystem access.

Cloud sync exists behind a trait (`EventStore`), not baked into the ledger core, and is entirely opt-in. The local encrypted log is always the source of truth regardless of whether Supabase is configured or reachable.

The project still avoids authentication systems, multi-tenant infrastructure, and heavy frameworks. Following the YAGNI principle ("You Aren't Gonna Need It"), features are only introduced when they provide clear value — sync earned its place because a second device reading the same ledger is a real, near-term need, not a hypothetical one.

---

## Testing

The project includes automated tests covering encryption round-trips, wrong-passphrase rejection, storage, event projections, summary calculations, balance math, subscription frequency math, and name normalization.

Run the test suite with:

```bash
cargo test
```

Build the project with:

```bash
cargo build --release
```

Run the application using:

```bash
./target/release/ft summary
```

At the time of writing, the entire V3 test suite passes successfully (17 tests).

---

## Future Direction

The current focus is keeping the ledger reliable, understandable, and useful. Potential future additions include budget ceilings, multi-currency support, cash flow forecasting, CSV/PDF export for paperwork, split expenses, a terminal dashboard (TUI), and anomaly flagging on unusual transactions.

A read-only mobile companion, backed by the same Supabase sync introduced in V3, is also on the table — quick-capture on the go, full history and queries staying on the desktop CLI.

Features that are intentionally not planned include full multi-user support, bank account integrations, and heavy frameworks. The goal is to remain a small, maintainable tool that can be understood in a single sitting.

---

## Current Status

**Version:** V3

**Architecture:** Event Sourced, Command/Query separated at the type level

**Storage:** Encrypted, append-only local log (Argon2id + XChaCha20-Poly1305) with optional Supabase sync

**Money Representation:** Exact Decimal Arithmetic, validated at construction (`Money` newtype)

**Testing:** Passing (17 tests)

**Scope:** Personal Finance Ledger

Built to learn Rust.

Kept small on purpose.
