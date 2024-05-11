use crate::types::TransactionType;

use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub enum ClientErr {
    AccountLocked,
    InsufficientFunds,
    DisputedTransactionNotFound,
    AlreadyProcessed,
}

#[derive(Debug, Default)]
pub struct ClientAccount {
    client: u16,
    pub(crate) available: Decimal,
    pub(crate) held: Decimal,
    pub(crate) total: Decimal,
    pub(crate) locked: bool,

    processed_tx: HashMap<u32, TransactionType>,
    under_dispute: HashSet<u32>,
}

impl ClientAccount {
    /// Constructs a new [`ClientAccount`] with the given client ID.
    pub fn new(client: u16) -> Self {
        Self {
            client,
            ..Default::default()
        }
    }

    /// True if the account is locked.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Process the transaction.
    pub fn process_transaction(&mut self, tx: TransactionType) -> Result<(), ClientErr> {
        if self.is_locked() {
            return Err(ClientErr::AccountLocked);
        }

        let tx_id = tx.transaction_id();
        if self.processed_tx.contains_key(&tx_id) {
            return Err(ClientErr::AlreadyProcessed);
        }

        match tx {
            TransactionType::Deposit { amount, .. } => self.handle_deposit(amount)?,
            TransactionType::Withdrawal { amount, .. } => self.handle_withdraw(amount)?,
            TransactionType::Dispute { tx, .. } => self.handle_dispute(tx)?,
            TransactionType::Resolve { tx, .. } => self.handle_resolve(tx)?,
            TransactionType::Chargeback { tx, .. } => self.handle_chargeback(tx)?,
        }

        self.processed_tx.insert(tx_id, tx);
        Ok(())
    }

    fn handle_deposit(&mut self, amount: Decimal) -> Result<(), ClientErr> {
        log::debug!("[client {}] handle_deposit {amount} ", self.client);

        self.available += amount;
        self.total += amount;

        Ok(())
    }

    fn handle_withdraw(&mut self, amount: Decimal) -> Result<(), ClientErr> {
        log::debug!("[client {}] handle_withdraw {amount}", self.client);
        if self.available < amount {
            return Err(ClientErr::InsufficientFunds);
        }

        self.available -= amount;
        self.total -= amount;
        Ok(())
    }

    fn handle_dispute(&mut self, tx: u32) -> Result<(), ClientErr> {
        log::debug!("[client {}] handle_dispute {tx}", self.client);

        let tx = self
            .processed_tx
            .get(&tx)
            .ok_or(ClientErr::DisputedTransactionNotFound)?;

        if let TransactionType::Deposit { amount, .. } = tx {
            self.available -= amount;
            self.held += amount;

            self.under_dispute.insert(tx.transaction_id());
        }

        Ok(())
    }

    fn handle_resolve(&mut self, tx: u32) -> Result<(), ClientErr> {
        log::debug!("[client {}] handle_resolve {tx}", self.client);

        let disputed_tx = self
            .processed_tx
            .get(&tx)
            .ok_or(ClientErr::DisputedTransactionNotFound)?;

        // Tx must be marked as disputed to resolve it.
        if !self.under_dispute.remove(&disputed_tx.transaction_id()) {
            return Err(ClientErr::DisputedTransactionNotFound);
        }

        if let TransactionType::Deposit { amount, .. } = disputed_tx {
            self.available += amount;
            self.held -= amount;
        }

        Ok(())
    }

    fn handle_chargeback(&mut self, tx: u32) -> Result<(), ClientErr> {
        log::debug!("[client {}] handle_chargeback {tx}", self.client);

        let disputed_tx = self
            .processed_tx
            .get(&tx)
            .ok_or(ClientErr::DisputedTransactionNotFound)?;

        // Tx must be marked as disputed to chargeback it.
        if !self.under_dispute.remove(&disputed_tx.transaction_id()) {
            return Err(ClientErr::DisputedTransactionNotFound);
        }

        if let TransactionType::Deposit { amount, .. } = disputed_tx {
            self.held -= amount;
            self.total -= amount;
            self.locked = true;
        }

        Ok(())
    }
}
