use crate::command::{ListKind, Query};
use crate::models::{Event, EventRecord};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Default, Debug)]
pub struct Summary {
    pub total_income: Decimal,
    pub total_expenses: Decimal,
    pub category_breakdown: HashMap<String, Decimal>,
    pub subscriptions: HashMap<String, Decimal>,
}

impl Summary {
    pub fn net_savings(&self) -> Decimal {
        self.total_income - self.total_expenses
    }
}

fn abs(v: Decimal) -> Decimal {
    if v.is_sign_negative() {
        -v
    } else {
        v
    }
}

fn fmt_map(f: &mut fmt::Formatter<'_>, title: &str, map: &HashMap<String, Decimal>) -> fmt::Result {
    if map.is_empty() {
        return Ok(());
    }
    writeln!(f, "\n{title}:")?;
    let mut items: Vec<_> = map.iter().collect();
    items.sort_by(|a, b| a.0.cmp(b.0));
    for (k, v) in items {
        writeln!(f, "  {k}: ₹{v:.2}")?;
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

/// IDs of events that a later `Reversed` marker points at. Every fold below
/// skips these, plus the `Reversed` markers themselves, so undone events
/// never contribute to totals but are never erased from the underlying log.
fn reversed_ids<'a>(events: impl Iterator<Item = &'a EventRecord>) -> HashSet<u64> {
    events
        .filter_map(|r| match &r.event {
            Event::Reversed { original_id, .. } => Some(*original_id),
            _ => None,
        })
        .collect()
}

pub fn compute_summary<'a>(events: impl Iterator<Item = &'a EventRecord> + Clone) -> Summary {
    let reversed = reversed_ids(events.clone());
    events
        .filter(|r| !reversed.contains(&r.id) && !matches!(r.event, Event::Reversed { .. }))
        .fold(Summary::default(), |mut acc, record| {
            match &record.event {
                Event::Expense {
                    amount, category, ..
                } => {
                    acc.total_expenses += amount.as_decimal();
                    *acc.category_breakdown.entry(category.clone()).or_default() +=
                        amount.as_decimal();
                }
                Event::Income { amount, .. } => acc.total_income += amount.as_decimal(),
                Event::SubscriptionCharged {
                    amount, service, ..
                } => {
                    acc.total_expenses += amount.as_decimal();
                    *acc.category_breakdown.entry(service.clone()).or_default() +=
                        amount.as_decimal();
                    *acc.subscriptions.entry(service.clone()).or_default() += amount.as_decimal();
                }
                _ => {}
            }
            acc
        })
}

pub fn compute_balances<'a>(
    events: impl Iterator<Item = &'a EventRecord> + Clone,
) -> HashMap<String, Decimal> {
    let reversed = reversed_ids(events.clone());
    events
        .filter(|r| !reversed.contains(&r.id))
        .fold(HashMap::new(), |mut acc, record| {
            match &record.event {
                Event::LoanGiven { amount, person, .. } => {
                    *acc.entry(person.clone()).or_default() += amount.as_decimal()
                }
                Event::RepaymentReceived { amount, person } => {
                    *acc.entry(person.clone()).or_default() -= amount.as_decimal()
                }
                Event::LoanTaken { amount, person, .. } => {
                    *acc.entry(person.clone()).or_default() -= amount.as_decimal()
                }
                Event::RepaymentMade { amount, person } => {
                    *acc.entry(person.clone()).or_default() += amount.as_decimal()
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
                || contains_ci(&amount.as_decimal().to_string(), q)
                || contains_ci(category, q)
                || contains_ci(description, q)
        }
        Event::Income {
            amount,
            source,
            description,
        } => {
            contains_ci("income", q)
                || contains_ci(&amount.as_decimal().to_string(), q)
                || contains_ci(source, q)
                || contains_ci(description, q)
        }
        Event::LoanGiven {
            amount,
            person,
            description,
        } => {
            contains_ci("loan given", q)
                || contains_ci(&amount.as_decimal().to_string(), q)
                || contains_ci(person, q)
                || contains_ci(description, q)
        }
        Event::LoanTaken {
            amount,
            person,
            description,
        } => {
            contains_ci("loan taken", q)
                || contains_ci(&amount.as_decimal().to_string(), q)
                || contains_ci(person, q)
                || contains_ci(description, q)
        }
        Event::RepaymentReceived { amount, person } => {
            contains_ci("repayment received", q)
                || contains_ci(&amount.as_decimal().to_string(), q)
                || contains_ci(person, q)
        }
        Event::RepaymentMade { amount, person } => {
            contains_ci("repayment made", q)
                || contains_ci(&amount.as_decimal().to_string(), q)
                || contains_ci(person, q)
        }
        Event::SubscriptionCreated {
            amount,
            service,
            frequency,
        } => {
            contains_ci("subscription", q)
                || contains_ci(&amount.as_decimal().to_string(), q)
                || contains_ci(service, q)
                || contains_ci(frequency, q)
        }
        Event::SubscriptionCharged {
            amount, service, ..
        } => {
            contains_ci("subscription charge", q)
                || contains_ci(&amount.as_decimal().to_string(), q)
                || contains_ci(service, q)
        }
        Event::Reversed { reason, .. } => {
            contains_ci("reversed", q) || reason.as_deref().is_some_and(|r| contains_ci(r, q))
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
            | (ListKind::Subscriptions, Event::SubscriptionCharged { .. })
    )
}

pub fn execute(events: &[EventRecord], query: Query) {
    match query {
        Query::History => {
            let reversed = reversed_ids(events.iter());
            for record in events {
                if let Event::Reversed { .. } = record.event {
                    println!("{record}");
                    continue;
                }
                if reversed.contains(&record.id) {
                    println!("{record}  [REVERSED]");
                } else {
                    println!("{record}");
                }
            }
        }
        Query::Summary { .. } => unreachable!("Summary is handled directly in main.rs"),
        Query::Owed => {
            println!("--- People who owe you ---");
            let mut balances: Vec<_> = compute_balances(events.iter())
                .into_iter()
                .filter(|(_, v)| v.is_sign_positive())
                .collect();
            balances.sort_by(|a, b| a.0.cmp(&b.0));
            for (person, amt) in balances {
                println!("{person}: ₹{amt:.2}");
            }
        }
        Query::Debts => {
            println!("--- Money you owe ---");
            let mut balances: Vec<_> = compute_balances(events.iter())
                .into_iter()
                .filter(|(_, v)| v.is_sign_negative())
                .collect();
            balances.sort_by(|a, b| a.0.cmp(&b.0));
            for (person, amt) in balances {
                println!("{person}: ₹{:.2}", abs(amt));
            }
        }
        Query::List { kind } => {
            println!("--- List ---");
            for record in events.iter().filter(|r| list_matches(kind, &r.event)) {
                println!("{record}");
            }
        }
        Query::Find { term } => {
            println!("--- Search Results for '{term}' ---");
            let matches = find_events(events, &term);
            if matches.is_empty() {
                println!("No matches found.");
                return;
            }
            for r in &matches {
                println!("{r}");
            }

            let q = term.to_lowercase();
            let balances = compute_balances(events.iter());
            if let Some((person, bal)) = balances
                .iter()
                .find(|(person, _)| person.to_lowercase() == q)
            {
                println!("\n--- Balance for '{person}' ---");
                println!(
                    "Current Balance: ₹{:.2} ({})",
                    abs(*bal),
                    if bal.is_sign_positive() {
                        "owed to you"
                    } else {
                        "you owe"
                    }
                );
            }

            let net_flow = matches.iter().fold(Decimal::ZERO, |acc, r| match &r.event {
                Event::Expense { amount, .. } | Event::SubscriptionCharged { amount, .. } => {
                    acc - amount.as_decimal()
                }
                Event::Income { amount, .. } => acc + amount.as_decimal(),
                _ => acc,
            });
            println!("Net Cash Impact: ₹{net_flow:.2}");
        }
    }
}

pub fn filtered_summary(
    events: &[EventRecord],
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
) -> Summary {
    let filtered = events.iter().filter(|r| {
        start.as_ref().map_or(true, |s| r.timestamp >= *s)
            && end.as_ref().map_or(true, |e| r.timestamp < *e)
    });
    compute_summary(filtered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Money;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    fn rec(id: u64, event: Event) -> EventRecord {
        EventRecord {
            id,
            timestamp: NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc(),
            event,
        }
    }

    fn money(cents: i64) -> Money {
        Money::new(Decimal::new(cents, 2)).unwrap()
    }

    #[test]
    fn test_compute_balances() {
        let events = vec![
            rec(
                1,
                Event::LoanGiven {
                    amount: money(5700),
                    person: "Sinan".into(),
                    description: "lunch".into(),
                },
            ),
            rec(
                2,
                Event::RepaymentReceived {
                    amount: money(2000),
                    person: "Sinan".into(),
                },
            ),
            rec(
                3,
                Event::LoanTaken {
                    amount: money(10000),
                    person: "Abijith".into(),
                    description: "emergency".into(),
                },
            ),
        ];
        let balances = compute_balances(events.iter());
        assert_eq!(balances.get("Sinan"), Some(&Decimal::new(3700, 2)));
        assert_eq!(balances.get("Abijith"), Some(&Decimal::new(-10000, 2)));
    }

    #[test]
    fn test_summary_with_subscription_charge() {
        let events = vec![
            rec(
                1,
                Event::Expense {
                    amount: money(20000),
                    category: "Food".into(),
                    description: "burger".into(),
                },
            ),
            rec(
                2,
                Event::Income {
                    amount: money(50000),
                    source: "Father".into(),
                    description: "fees".into(),
                },
            ),
            rec(
                3,
                Event::SubscriptionCreated {
                    amount: money(8990),
                    service: "Spotify".into(),
                    frequency: "monthly".into(),
                },
            ),
            rec(
                4,
                Event::SubscriptionCharged {
                    subscription_id: 3,
                    amount: money(8990),
                    service: "Spotify".into(),
                },
            ),
        ];
        let s = compute_summary(events.iter());
        assert_eq!(s.total_expenses, Decimal::new(28990, 2));
        assert_eq!(s.total_income, Decimal::new(50000, 2));
        assert_eq!(s.subscriptions.get("Spotify"), Some(&Decimal::new(8990, 2)));
    }

    #[test]
    fn reversed_event_excluded_from_summary() {
        let events = vec![
            rec(
                1,
                Event::Expense {
                    amount: money(20000),
                    category: "Food".into(),
                    description: "burger".into(),
                },
            ),
            rec(
                2,
                Event::Reversed {
                    original_id: 1,
                    reason: None,
                },
            ),
        ];
        let s = compute_summary(events.iter());
        assert_eq!(s.total_expenses, Decimal::ZERO);
    }
}
