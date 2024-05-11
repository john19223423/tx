use crate::{client::ClientAccount, types::TransactionType};

use std::collections::HashMap;

pub struct PaymentEngine {
    accounts: HashMap<u16, ClientAccount>,
}

impl PaymentEngine {
    /// Constructs a new [`PaymentEngine`].
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    /// Process the given transaction.
    pub fn process_transaction(&mut self, tx: TransactionType) {
        let client_id = tx.client_id();

        let account = self
            .accounts
            .entry(client_id)
            .or_insert_with(|| ClientAccount::new(client_id));

        if let Err(err) = account.process_transaction(tx) {
            log::error!("[{}] Error processing transaction: {:?}", client_id, err);
        }
    }

    /// Serialize the current state of the accounts.
    pub fn serialize(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut writer = csv::Writer::from_writer(std::io::stdout());

        writer.write_record(["client", "available", "held", "total", "locked"])?;

        for (client, account) in &self.accounts {
            writer
                .write_record(&[
                    client.to_string(),
                    account.available.to_string(),
                    account.held.to_string(),
                    account.total.to_string(),
                    account.locked.to_string(),
                ])
                .unwrap();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TransactionType;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn test_multiaccount_deposit() {
        let mut engine = PaymentEngine::new();

        let tx = TransactionType::Deposit {
            client: 1,
            tx: 1,
            amount: Decimal::from_str("1.0").unwrap(),
        };

        engine.process_transaction(tx);

        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.available, Decimal::from_str("1.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("1.0").unwrap());
        assert_eq!(account.locked, false);

        let tx = TransactionType::Deposit {
            client: 2,
            tx: 2,
            amount: Decimal::from_str("4.0").unwrap(),
        };

        engine.process_transaction(tx);

        // Account 1 unaffected.
        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.available, Decimal::from_str("1.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("1.0").unwrap());
        assert_eq!(account.locked, false);

        // Account 2 updated.
        let account = engine.accounts.get(&2).unwrap();
        assert_eq!(account.available, Decimal::from_str("4.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("4.0").unwrap());
        assert_eq!(account.locked, false);
    }

    #[test]
    fn test_multiaccount_withdraw() {
        let mut engine = PaymentEngine::new();

        let tx = TransactionType::Deposit {
            client: 1,
            tx: 1,
            amount: Decimal::from_str("1.0").unwrap(),
        };
        engine.process_transaction(tx);
        let tx = TransactionType::Deposit {
            client: 2,
            tx: 2,
            amount: Decimal::from_str("4.0").unwrap(),
        };
        engine.process_transaction(tx);

        let tx = TransactionType::Withdrawal {
            client: 1,
            tx: 3,
            amount: Decimal::from_str("0.5").unwrap(),
        };
        engine.process_transaction(tx);
        let tx = TransactionType::Withdrawal {
            client: 2,
            tx: 4,
            amount: Decimal::from_str("1.0").unwrap(),
        };
        engine.process_transaction(tx);

        let account = engine.accounts.get(&1).unwrap();
        assert_eq!(account.available, Decimal::from_str("0.5").unwrap());
        assert_eq!(account.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("0.5").unwrap());
        assert_eq!(account.locked, false);

        let account = engine.accounts.get(&2).unwrap();
        assert_eq!(account.available, Decimal::from_str("3.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("3.0").unwrap());
        assert_eq!(account.locked, false);
    }
}
