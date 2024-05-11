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
