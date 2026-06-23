# Personal Finance Ledger

A lean, event-sourced personal finance ledger written in Rust.

Track money intentionally. Learn Rust deeply. Own your data.

---

## Why This Project Exists

Personal Finance Ledger started as a learning project. I wanted to learn Rust by building something I would genuinely use every day rather than creating another tutorial application, toy project, or CRUD example that would be abandoned after a weekend.

At the same time, I needed a better way to track my university finances. Most finance applications optimize for convenience, automation, and reducing user interaction as much as possible. While that approach works well for many people, I found that manually recording expenses made me far more aware of where my money was going.

This project embraces what I call **Intentional Friction**. Instead of hiding financial decisions behind automatic categorization, bank integrations, and background synchronization, every transaction is entered deliberately. The terminal is not a limitation; it is part of the experience.

The result is a lightweight, local-first finance ledger that helps me learn Rust while remaining genuinely useful in daily life.

---

## Why You Might Like It

This project is not trying to compete with commercial finance software. Instead, it targets a very specific type of user.

If you enjoy terminal workflows, prefer owning your own data, want a deeper understanding of your spending habits, or are learning Rust through practical projects, this ledger may be exactly what you're looking for. It stores everything locally, uses plain JSON files, has no accounts, no cloud synchronization, no subscriptions, and no hidden telemetry.

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

One rule drives the entire architecture:

```text
Commands create Events.
Queries read Events.
```

If something creates a new financial fact, it becomes an event. If something only reads existing information, it becomes a query.

---

## Architecture

The architecture intentionally remains simple.

```text
CLI
 │
 ▼
Parser
 │
 ▼
Command
 │
 ▼
Ledger
 │
 ▼
Event
 │
 ▼
JSON Storage
```

Commands are responsible for creating events. Queries are responsible for reading and projecting information from those events. The ledger itself owns the event history and acts as the source of truth.

Because the project uses event sourcing, there is no separate balance table, debt table, or summary table. Everything is derived from the recorded history.

---

## Storage and Reliability

The ledger stores data locally as JSON. Human-readable storage makes debugging easier, keeps the project approachable, and avoids introducing database complexity before it is actually needed.

Version 2 focuses heavily on reliability. Saves are atomic, meaning the application writes to a temporary file and only replaces the original once the operation completes successfully. A backup file is also maintained automatically so that corrupted data can be recovered when possible.

Financial amounts are stored using `rust_decimal::Decimal` rather than floating-point numbers. This avoids precision issues and ensures that values such as `0.1 + 0.2` behave exactly as expected.

The ledger also supports undoing the most recently recorded event, making accidental entries easy to correct.

---

## Features

The ledger currently supports recording income, expenses, loans given to other people, loans taken from other people, repayments, and recurring subscriptions. Each of these actions becomes an immutable event in the ledger.

Queries allow you to view event history, generate summaries, inspect outstanding debts, review money owed to you, search the ledger, and filter information by category or event type.

All balances and reports are derived directly from the event history rather than being stored separately.

---

## Recording Financial Events

The following commands create new events inside the ledger.

| Action            | Example                                     |
| ----------------- | ------------------------------------------- |
| Expense           | `ledger expense 200 food burger_king`       |
| Income            | `ledger income 15000 father semester_fees`  |
| Lend Money        | `ledger lend 500 alex lunch`               |
| Borrow Money      | `ledger borrow 1000 max emergency_cash` |
| Receive Repayment | `ledger receive 200 alex`                  |
| Repay Debt        | `ledger repay 400 max`                  |
| Subscription      | `ledger subscribe 89.9 spotify monthly`     |
| Undo Last Event   | `ledger undo`                               |

Every command immediately creates a new event and persists it to storage.

---

## Querying the Ledger

Queries never modify data. They simply read information derived from the event history.

| Query                            | Description                   |
| -------------------------------- | ----------------------------- |
| `ledger history`                 | Show every recorded event     |
| `ledger summary`                 | Display financial overview    |
| `ledger summary 2026-01 2026-12` | Summary for a date range      |
| `ledger owed`                    | Show people who owe you money |
| `ledger debts`                   | Show money you owe others     |
| `ledger list expenses`           | List expense events           |
| `ledger list income`             | List income events            |
| `ledger list loans`              | List loan-related events      |
| `ledger list subscriptions`      | List subscriptions            |
| `ledger find alex`               | Search for a person           |
| `ledger find food`               | Search by category            |
| `ledger find burger`             | Search by merchant            |
| `ledger find spotify`            | Search by subscription        |

The `summary` command is intended to be the flagship query. It provides an overview of income, expenses, savings, category breakdowns, subscriptions, and debt statistics.

---

## Example Session

A typical session might look like this:

```bash
ledger income 15000 father semester_fees

ledger expense 120 food burger_king

ledger lend 500 alex lunch

ledger receive 200 alex

ledger summary
```

Which could produce something similar to:

```text
Income:   ₹15000.00
Expenses: ₹120.00
Net:      ₹14880.00

Food: ₹120.00

Owed To You: ₹300.00
You Owe: ₹0.00
```

The balance owed by alex is never stored directly. It is calculated from the recorded loan and repayment events.

---

## Technical Decisions

Several design decisions were made intentionally.

JSON was chosen because it is simple, transparent, and easy to inspect manually. Event sourcing was chosen because it preserves complete financial history while avoiding duplicated state. Decimal arithmetic was chosen because money should never be represented using floating-point values.

The project deliberately avoids databases, cloud services, authentication systems, asynchronous runtimes, and other infrastructure that would add complexity without improving the core experience.

Following the YAGNI principle ("You Aren't Gonna Need It"), features are only introduced when they provide clear value.

---

## Testing

The project includes automated tests covering storage, serialization, event projections, and summary calculations.

Run the test suite with:

```bash
cargo test
```

Build the project with:

```bash
cargo build
```

Run the application using:

```bash
cargo run -- summary
```

At the time of writing, the entire V2 test suite passes successfully.

---

## Future Direction

The current focus is keeping the ledger reliable, understandable, and useful. Potential future additions include monthly reports, budget tracking, semester-based finance tracking, trip tracking, CSV export, and richer statistics.

Features that are intentionally not planned include cloud synchronization, multi-user support, mobile applications, and heavy frameworks. The goal is to remain a small, maintainable tool that can be understood in a single sitting.

---

## Current Status

**Version:** V2

**Architecture:** Event Sourced

**Storage:** JSON + Atomic Saves + Backup Recovery

**Money Representation:** Exact Decimal Arithmetic

**Testing:** Passing

**Scope:** Personal Finance Ledger

Built to learn Rust.

Kept small on purpose.
