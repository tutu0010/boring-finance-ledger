use clap::Parser;
use personal_finance_ledger::command::{Cli, Command};
use personal_finance_ledger::ledger::Ledger;
use personal_finance_ledger::query;
use personal_finance_ledger::storage;
use std::env;
use std::process;

fn main() {
    let cli = Cli::parse();
    let db_path = env::var("LEDGER_FILE")
        .unwrap_or_else(|_| storage::default_path().to_string_lossy().to_string());

    let loaded = match storage::load(&db_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to load ledger: {}", e);
            process::exit(1);
        }
    };

    if loaded.recovered_from_backup {
        eprintln!("Warning: main ledger file was unreadable, recovered from backup.");
        if let Err(e) = storage::save(&db_path, &loaded.events) {
            eprintln!("Failed to repair ledger after recovery: {}", e);
            process::exit(1);
        }
    }

    let mut app = Ledger::new(db_path, loaded.events);

    match cli.command {
        Command::Expense { .. }
        | Command::Income { .. }
        | Command::Lend { .. }
        | Command::Borrow { .. }
        | Command::Repay { .. }
        | Command::Receive { .. }
        | Command::Subscribe { .. } => {
            if let Err(e) = app.record(cli.command) {
                eprintln!("Execution failed: {}", e);
                process::exit(1);
            }
        }
        Command::Undo => {
            if let Err(e) = app.undo() {
                eprintln!("Execution failed: {}", e);
                process::exit(1);
            }
        }
        Command::History => query::execute(app.events(), Command::History),
        Command::Summary { start, end } => {
            let bounds = Command::Summary { start, end }.summary_bounds();
            match bounds {
                Ok(Some((start, end))) => {
                    let summary = query::filtered_summary(app.events(), start, end);
                    println!("{}", summary);
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
                        println!("  Owed to you: ₹{:.2}", owed_to_you);
                        println!("  You owe:     ₹{:.2}", you_owe);
                    }
                }
                Ok(None) => unreachable!(),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }
        Command::Owed | Command::Debts | Command::List { .. } | Command::Find { .. } => {
            query::execute(app.events(), cli.command)
        }
    }
}
