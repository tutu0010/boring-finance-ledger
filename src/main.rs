use clap::Parser;
use personal_finance_ledger::command::{Cli, Dispatch, Query};
use personal_finance_ledger::ledger::{Ledger, RecordOutcome, UndoOutcome};
use personal_finance_ledger::query;
use personal_finance_ledger::store::local::EncryptedJsonlStore;
use personal_finance_ledger::store::{default_data_dir, remote, EventStore};
use std::process;

fn prompt_passphrase(confirm: bool) -> std::io::Result<String> {
    loop {
        let pass = rpassword::prompt_password("Ledger passphrase: ")?;
        if pass.is_empty() {
            eprintln!("Passphrase cannot be empty.");
            continue;
        }
        if confirm {
            let confirm_pass = rpassword::prompt_password("Confirm passphrase: ")?;
            if pass != confirm_pass {
                eprintln!("Passphrases did not match. Try again.\n");
                continue;
            }
        }
        return Ok(pass);
    }
}

fn main() {
    let cli = Cli::parse();
    let data_dir = default_data_dir();

    let is_new_setup = EncryptedJsonlStore::is_new_setup(&data_dir);
    if is_new_setup {
        println!("No ledger found at {}.", data_dir.display());
        println!("Set a passphrase to encrypt it — you'll need this every time you run `ft`.");
    }

    let passphrase = match prompt_passphrase(is_new_setup) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to read passphrase: {e}");
            process::exit(1);
        }
    };

    let opened = match EncryptedJsonlStore::open(&data_dir, &passphrase) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Failed to unlock ledger: {e}");
            process::exit(1);
        }
    };

    if opened.migrated_legacy_events > 0 {
        eprintln!(
            "Migrated {} event(s) from the old plaintext ledger.json into the new encrypted store.",
            opened.migrated_legacy_events
        );
    }

    let mut store = opened.store;
    let events = match store.load() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to load ledger: {e}");
            process::exit(1);
        }
    };

    let mut app = Ledger::new(store, events);

    match app.materialize_due_subscriptions() {
        Ok(charges) if !charges.is_empty() => {
            eprintln!("Applied {} due subscription charge(s).", charges.len());
        }
        Ok(_) => {}
        Err(e) => eprintln!("Warning: could not process subscription charges: {e}"),
    }

    let sync_report = remote::sync_pending(app.events(), &data_dir);
    if let Some(err) = &sync_report.error {
        eprintln!(
            "Supabase sync incomplete ({} event(s) still pending): {err}",
            sync_report.pending
        );
    } else if sync_report.pushed > 0 {
        eprintln!("Synced {} event(s) to Supabase.", sync_report.pushed);
    }

    let dispatch = match Dispatch::try_from(cli.action) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    match dispatch {
        Dispatch::Write(cmd) => match app.record(cmd) {
            Ok(RecordOutcome::Recorded(record)) => println!("Recorded: {record}"),
            Err(e) => {
                eprintln!("Execution failed: {e}");
                process::exit(1);
            }
        },
        Dispatch::Undo => match app.undo() {
            Ok(UndoOutcome::Reversed { original, .. }) => {
                println!("Reversed #{}: {}", original.id, original.event);
            }
            Err(e) => {
                eprintln!("Execution failed: {e}");
                process::exit(1);
            }
        },
        Dispatch::Read(Query::Summary { start, end }) => {
            let summary = query::filtered_summary(app.events(), start, end);
            println!("{summary}");
            let balances = query::compute_balances(app.events().iter());
            if !balances.is_empty() {
                let owed_to_you: rust_decimal::Decimal = balances
                    .values()
                    .filter(|v| v.is_sign_positive())
                    .copied()
                    .sum();
                let you_owe: rust_decimal::Decimal = balances
                    .values()
                    .filter(|v| v.is_sign_negative())
                    .map(|v| v.abs())
                    .sum();
                println!("\nDebt Statistics:");
                println!("  Owed to you: ₹{owed_to_you:.2}");
                println!("  You owe:     ₹{you_owe:.2}");
            }
        }
        Dispatch::Read(query) => query::execute(app.events(), query),
    }
}
