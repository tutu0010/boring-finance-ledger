use crate::command::{ListKind, QueryType};
use crate::models::{Event, EventRecord};
use std::collections::HashMap;
use std::fmt;

#[derive(Default, Debug)]
pub struct Summary {
    pub total_income: f64,
    pub total_expenses: f64,
    pub category_breakdown: HashMap<String, f64>,
    pub subscriptions: HashMap<String, f64>,
}

impl Summary {
    pub fn net_savings(&self) -> f64 {
        self.total_income - self.total_expenses
    }
}

fn fmt_map(f: &mut fmt::Formatter<'_>, title: &str, map: &HashMap<String, f64>) -> fmt::Result {
    if map.is_empty() {
        return Ok(());
    }

    writeln!(f, "\n{}:", title)?;
    let mut items: Vec<_> = map.iter().collect();
    items.sort_by(|a, b| a.0.cmp(b.0));
    for (k, v) in items {
        writeln!(f, "  {}: ₹{:.2}", k, v)?;
    }
    Ok(())
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "--- Summary ---")?;
        writeln!(f, "Income:   ₹{:.2}", self.total_income)?;
        writeln!(f, "Expenses: ₹{:.2}", self.total_expenses)?;
        writeln!(f, "Net:      ₹{:.2}", self.net_savings())?;
        fmt_map(f, "Category Breakdown", &self.category_breakdown)?;
        fmt_map(f, "Subscription Costs", &self.subscriptions)?;
        Ok(())
    }
}

pub fn compute_summary<'a>(events: impl Iterator<Item = &'a EventRecord>) -> Summary {
    events.fold(Summary::default(), |mut acc, record| {
        match &record.event {
            Event::Expense {
                amount, category, ..
            } => {
                acc.total_expenses += amount;
                *acc.category_breakdown.entry(category.clone()).or_default() += amount;
            }
            Event::Income { amount, .. } => acc.total_income += amount,
            Event::SubscriptionCreated {
                amount, service, ..
            } => {
                *acc.subscriptions.entry(service.clone()).or_default() += amount;
            }
            _ => {}
        }
        acc
    })
}

pub fn compute_balances<'a>(events: impl Iterator<Item = &'a EventRecord>) -> HashMap<String, f64> {
    events.fold(HashMap::new(), |mut acc, record| {
        match &record.event {
            Event::LoanGiven { amount, person, .. } => {
                *acc.entry(person.clone()).or_default() += amount;
            }
            Event::RepaymentReceived { amount, person } => {
                *acc.entry(person.clone()).or_default() += amount;
            }
            Event::LoanTaken { amount, person, .. } => {
                *acc.entry(person.clone()).or_default() -= amount;
            }
            Event::RepaymentMade { amount, person } => {
                *acc.entry(person.clone()).or_default() -= amount;
            }
            _ => {}
        }
        acc
    })
}

fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(needle)
}

fn matches_event(event: &Event, q: &str) -> bool {
    match event {
        Event::Expense {
            amount,
            category,
            description,
        } => {
            contains_ci("expense", q)
                || contains_ci(&amount.to_string(), q)
                || contains_ci(category, q)
                || contains_ci(description, q)
        }
        Event::Income {
            amount,
            source,
            description,
        } => {
            contains_ci("income", q)
                || contains_ci(&amount.to_string(), q)
                || contains_ci(source, q)
                || contains_ci(description, q)
        }
        Event::LoanGiven {
            amount,
            person,
            description,
        } => {
            contains_ci("loan given", q)
                || contains_ci(&amount.to_string(), q)
                || contains_ci(person, q)
                || contains_ci(description, q)
        }
        Event::LoanTaken {
            amount,
            person,
            description,
        } => {
            contains_ci("loan taken", q)
                || contains_ci(&amount.to_string(), q)
                || contains_ci(person, q)
                || contains_ci(description, q)
        }
        Event::RepaymentReceived { amount, person } => {
            contains_ci("repayment received", q)
                || contains_ci(&amount.to_string(), q)
                || contains_ci(person, q)
        }
        Event::RepaymentMade { amount, person } => {
            contains_ci("repayment made", q)
                || contains_ci(&amount.to_string(), q)
                || contains_ci(person, q)
        }
        Event::SubscriptionCreated {
            amount,
            service,
            frequency,
        } => {
            contains_ci("subscription", q)
                || contains_ci(&amount.to_string(), q)
                || contains_ci(service, q)
                || contains_ci(frequency, q)
        }
    }
}

pub fn find_events<'a>(events: &'a [EventRecord], query: &str) -> Vec<&'a EventRecord> {
    let q = query.to_lowercase();
    events
        .iter()
        .filter(|r| matches_event(&r.event, &q))
        .collect()
}

fn list_matches(kind: ListKind, event: &Event) -> bool {
    matches!(
        (kind, event),
        (ListKind::Expenses, Event::Expense { .. })
            | (ListKind::Income, Event::Income { .. })
            | (ListKind::Loans, Event::LoanGiven { .. })
            | (ListKind::Loans, Event::LoanTaken { .. })
            | (ListKind::Subscriptions, Event::SubscriptionCreated { .. })
    )
}

pub fn execute(events: &[EventRecord], query: QueryType) {
    match query {
        QueryType::History => {
            for record in events {
                println!("{}", record);
            }
        }
        QueryType::Summary { start, end } => {
            let start = start.clone();
            let end = end.clone();
            let filtered: Vec<_> = events
                .iter()
                .filter(|r| {
                    start.as_ref().map_or(true, |s| r.timestamp >= *s)
                        && end.as_ref().map_or(true, |e| r.timestamp < *e)
                })
                .collect();

            let summary = compute_summary(filtered.iter().copied());
            println!("{}", summary);

            let balances = compute_balances(filtered.iter().copied());
            if !balances.is_empty() {
                let owed_to_you: f64 = balances.values().filter(|v| **v > 0.0).sum();
                let you_owe: f64 = balances
                    .values()
                    .filter(|v| **v < 0.0)
                    .map(|v| v.abs())
                    .sum();
                println!("\nDebt Statistics:");
                println!("  Owed to you: ₹{:.2}", owed_to_you);
                println!("  You owe:     ₹{:.2}", you_owe);
            }
        }
        QueryType::Owed => {
            println!("--- People who owe you ---");
            let mut balances: Vec<_> = compute_balances(events.iter())
                .into_iter()
                .filter(|(_, v)| *v > 0.0)
                .collect();
            balances.sort_by(|a, b| a.0.cmp(&b.0));
            for (person, amt) in balances {
                println!("{}: ₹{:.2}", person, amt);
            }
        }
        QueryType::Debts => {
            println!("--- Money you owe ---");
            let mut balances: Vec<_> = compute_balances(events.iter())
                .into_iter()
                .filter(|(_, v)| *v < 0.0)
                .collect();
            balances.sort_by(|a, b| a.0.cmp(&b.0));
            for (person, amt) in balances {
                println!("{}: ₹{:.2}", person, amt.abs());
            }
        }
        QueryType::List(kind) => {
            println!("--- List ---");
            for record in events.iter().filter(|r| list_matches(kind, &r.event)) {
                println!("{}", record);
            }
        }
        QueryType::Find(term) => {
            println!("--- Search Results for '{}' ---", term);
            let matches = find_events(events, &term);

            if matches.is_empty() {
                println!("No matches found.");
                return;
            }

            for r in &matches {
                println!("{}", r);
            }

            let q = term.to_lowercase();
            let balances = compute_balances(events.iter());
            if let Some((person, bal)) = balances
                .iter()
                .find(|(person, _)| person.to_lowercase() == q)
            {
                println!("\n--- Balance for '{}' ---", person);
                println!(
                    "Current Balance: ₹{:.2} ({})",
                    bal.abs(),
                    if *bal > 0.0 { "owed to you" } else { "you owe" }
                );
            }

            let net_flow = matches.iter().fold(0.0, |acc, r| match &r.event {
                Event::Expense { amount, .. } | Event::SubscriptionCreated { amount, .. } => {
                    acc - amount
                }
                Event::Income { amount, .. } => acc + amount,
                _ => acc,
            });

            println!("Net Cash Impact: ₹{:.2}", net_flow);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Event;

    #[test]
    fn test_compute_balances() {
        let events = vec![
            EventRecord::new(Event::LoanGiven {
                amount: 57.0,
                person: "Sinan".into(),
                description: "lunch".into(),
            }),
            EventRecord::new(Event::RepaymentReceived {
                amount: 20.0,
                person: "Sinan".into(),
            }),
            EventRecord::new(Event::LoanTaken {
                amount: 100.0,
                person: "Abijith".into(),
                description: "emergency".into(),
            }),
        ];
        let balances = compute_balances(events.iter());
        assert_eq!(balances.get("Sinan"), Some(&37.0));
        assert_eq!(balances.get("Abijith"), Some(&-100.0));
    }
}
