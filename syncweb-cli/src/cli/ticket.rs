use std::io::Read;

use anyhow::{Context, Result};

/// Resolve a ticket argument, reading the complete ticket from stdin when the
/// conventional `-` argument is used.
pub fn resolve_ticket(value: &str) -> Result<String> {
    if value != "-" {
        return Ok(value.to_owned());
    }

    read_ticket(std::io::stdin().lock())
}

fn read_ticket<R: Read>(mut reader: R) -> Result<String> {
    let mut input = String::new();
    reader.read_to_string(&mut input).context("read ticket from stdin")?;
    let ticket = input.trim();
    if ticket.is_empty() {
        anyhow::bail!("ticket read from stdin is empty");
    }
    Ok(ticket.to_owned())
}

pub fn resolve_tickets(values: &[String]) -> Result<Vec<String>> {
    values.iter().map(|value| resolve_ticket(value)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_ticket_arguments_are_unchanged() {
        assert_eq!(resolve_ticket("ticket-value").expect("resolve ticket"), "ticket-value");
    }

    #[test]
    fn ticket_lists_preserve_order() {
        let values = vec!["first".to_owned(), "second".to_owned()];
        assert_eq!(resolve_tickets(&values).expect("resolve tickets"), values);
    }

    #[test]
    fn stdin_ticket_is_trimmed() {
        assert_eq!(
            read_ticket("  ticket-value\n".as_bytes()).expect("read ticket"),
            "ticket-value"
        );
    }

    #[test]
    fn empty_stdin_ticket_is_rejected() {
        let error = read_ticket("\n".as_bytes()).expect_err("empty ticket should fail");
        assert!(error.to_string().contains("empty"));
    }
}
