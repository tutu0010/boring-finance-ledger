mod command;
mod errors;
mod ledger;
mod models;
mod query;
mod storage;

use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let db_path = env::var("LEDGER_FILE").unwrap_or_else(|_| "ledger.json".to_string());

    let events = match storage::load(&db_path) {
        Ok(evs) => evs,
        Err(e) => {
            eprintln!("Failed to load ledger: {}", e);
            process::exit(1);
        }
    };

    let mut app = ledger::Ledger::new(db_path, events);

    let action = match command::parse(&args) {
        Ok(act) => act,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    match action {
        command::Action::Command(cmd) => {
            if let Err(e) = app.record(cmd) {
                eprintln!("Execution failed: {}", e);
                process::exit(1);
            }
        }
        command::Action::Query(query) => query::execute(app.events(), query),
    }
}
