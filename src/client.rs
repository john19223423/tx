use crate::types::{TransactionType, PRECISION};

use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub enum ClientErr {
    AccountLocked,
    InsufficientFunds,
    DisputedTransactionNotFound,
    AlreadyProcessed,
}

#[derive(Debug)]
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
            available: Decimal::new(0, PRECISION),
            held: Decimal::new(0, PRECISION),
            total: Decimal::new(0, PRECISION),
            locked: false,
            processed_tx: HashMap::new(),
            under_dispute: HashSet::new(),
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

        match tx {
            TransactionType::Deposit {
                tx: tx_id, amount, ..
            } => {
                self.handle_deposit(tx_id, amount)?;
                self.processed_tx.insert(tx_id, tx);
            }
            TransactionType::Withdrawal {
                tx: tx_id, amount, ..
            } => {
                self.handle_withdraw(tx_id, amount)?;
                self.processed_tx.insert(tx_id, tx);
            }
            TransactionType::Dispute { tx, .. } => self.handle_dispute(tx)?,
            TransactionType::Resolve { tx, .. } => self.handle_resolve(tx)?,
            TransactionType::Chargeback { tx, .. } => self.handle_chargeback(tx)?,
        }

        Ok(())
    }

    fn handle_deposit(&mut self, tx: u32, amount: Decimal) -> Result<(), ClientErr> {
        log::debug!("[client {}] handle_deposit {amount} ", self.client);

        if self.processed_tx.contains_key(&tx) {
            return Err(ClientErr::AlreadyProcessed);
        }

        self.available += amount;
        self.total += amount;

        Ok(())
    }

    fn handle_withdraw(&mut self, tx: u32, amount: Decimal) -> Result<(), ClientErr> {
        log::debug!("[client {}] handle_withdraw {amount}", self.client);
        if self.processed_tx.contains_key(&tx) {
            return Err(ClientErr::AlreadyProcessed);
        }

        if self.available < amount {
            return Err(ClientErr::InsufficientFunds);
        }

        self.available -= amount;
        self.total -= amount;
        Ok(())
    }

    fn handle_dispute(&mut self, tx: u32) -> Result<(), ClientErr> {
        log::debug!("[client {}] handle_dispute {tx}", self.client);

        if self.under_dispute.contains(&tx) {
            return Err(ClientErr::AlreadyProcessed);
        }

        let tx = self
            .processed_tx
            .get(&tx)
            .ok_or(ClientErr::DisputedTransactionNotFound)?;

        log::debug!("[client {}] dispute found: {tx:?}", self.client);

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

#[cfg(test)]
mod tests {

    #[test]
    fn check_deposit() {
        // Valid deposit.
        let mut account = super::ClientAccount::new(1);
        let tx = super::TransactionType::Deposit {
            client: 1,
            tx: 1,
            amount: "1.0".parse().unwrap(),
        };

        account.process_transaction(tx.clone()).unwrap();
        assert_eq!(account.available, "1.0".parse().unwrap());
        assert_eq!(account.total, "1.0".parse().unwrap());

        // Duplicate deposit.
        account.process_transaction(tx.clone()).unwrap_err();
        assert_eq!(account.available, "1.0".parse().unwrap());
        assert_eq!(account.total, "1.0".parse().unwrap());

        // Second valid.
        let tx = super::TransactionType::Deposit {
            client: 1,
            tx: 2,
            amount: "1.0".parse().unwrap(),
        };
        account.process_transaction(tx.clone()).unwrap();
        assert_eq!(account.available, "2.0".parse().unwrap());
        assert_eq!(account.total, "2.0".parse().unwrap());
    }

    #[test]
    fn check_withdraw() {
        let mut account = super::ClientAccount::new(1);
        let tx = super::TransactionType::Deposit {
            client: 1,
            tx: 1,
            amount: "1.0".parse().unwrap(),
        };

        account.process_transaction(tx.clone()).unwrap();
        assert_eq!(account.available, "1.0".parse().unwrap());
        assert_eq!(account.total, "1.0".parse().unwrap());

        // Valid withdraw.
        let tx = super::TransactionType::Withdrawal {
            client: 1,
            tx: 2,
            amount: "0.5".parse().unwrap(),
        };
        account.process_transaction(tx.clone()).unwrap();
        assert_eq!(account.available, "0.5".parse().unwrap());
        assert_eq!(account.total, "0.5".parse().unwrap());

        // Duplicate withdraw.
        account.process_transaction(tx.clone()).unwrap_err();

        // Insufficient funds.
        let tx = super::TransactionType::Withdrawal {
            client: 1,
            tx: 3,
            amount: "1.0".parse().unwrap(),
        };
        account.process_transaction(tx.clone()).unwrap_err();
        assert_eq!(account.available, "0.5".parse().unwrap());
        assert_eq!(account.total, "0.5".parse().unwrap());
    }

    #[test]
    fn check_dispute_resolve_multiple_times() {
        env_logger::init();

        let mut account = super::ClientAccount::new(1);
        let tx = super::TransactionType::Deposit {
            client: 1,
            tx: 1,
            amount: "1.0".parse().unwrap(),
        };

        account.process_transaction(tx.clone()).unwrap();
        assert_eq!(account.available, "1.0".parse().unwrap());
        assert_eq!(account.total, "1.0".parse().unwrap());

        // Valid dispute.
        let tx = super::TransactionType::Dispute { client: 1, tx: 1 };
        account.process_transaction(tx.clone()).unwrap();
        assert_eq!(account.available, "0.0".parse().unwrap());
        assert_eq!(account.held, "1.0".parse().unwrap());
        assert_eq!(account.total, "1.0".parse().unwrap());

        // Already under dispute.
        let tx = super::TransactionType::Dispute { client: 1, tx: 1 };
        account.process_transaction(tx.clone()).unwrap_err();
        assert_eq!(account.available, "0.0".parse().unwrap());
        assert_eq!(account.held, "1.0".parse().unwrap());
        assert_eq!(account.total, "1.0".parse().unwrap());

        // Resolve.
        let tx = super::TransactionType::Resolve { client: 1, tx: 1 };
        account.process_transaction(tx.clone()).unwrap();
        assert_eq!(account.available, "1.0".parse().unwrap());
        assert_eq!(account.held, "0.0".parse().unwrap());
        assert_eq!(account.total, "1.0".parse().unwrap());

        // Already resolved.
        let tx = super::TransactionType::Resolve { client: 1, tx: 1 };
        account.process_transaction(tx.clone()).unwrap_err();
        assert_eq!(account.available, "1.0".parse().unwrap());
        assert_eq!(account.held, "0.0".parse().unwrap());
        assert_eq!(account.total, "1.0".parse().unwrap());
    }

    #[test]
    fn check_dispute_chargeback() {
        let mut account = super::ClientAccount::new(1);
        let tx = super::TransactionType::Deposit {
            client: 1,
            tx: 1,
            amount: "1.0".parse().unwrap(),
        };

        account.process_transaction(tx.clone()).unwrap();
        assert_eq!(account.available, "1.0".parse().unwrap());
        assert_eq!(account.total, "1.0".parse().unwrap());

        // Valid dispute.
        let tx = super::TransactionType::Dispute { client: 1, tx: 1 };
        account.process_transaction(tx.clone()).unwrap();
        assert_eq!(account.available, "0.0".parse().unwrap());
        assert_eq!(account.held, "1.0".parse().unwrap());
        assert_eq!(account.total, "1.0".parse().unwrap());

        // Cannot withdraw with insufficient funds under dispute.
        let tx = super::TransactionType::Withdrawal {
            client: 1,
            tx: 2,
            amount: "1.0".parse().unwrap(),
        };
        account.process_transaction(tx.clone()).unwrap_err();
        assert_eq!(account.available, "0.0".parse().unwrap());
        assert_eq!(account.held, "1.0".parse().unwrap());
        assert_eq!(account.total, "1.0".parse().unwrap());

        // Chargeback.
        let tx = super::TransactionType::Chargeback { client: 1, tx: 1 };
        account.process_transaction(tx.clone()).unwrap();
        assert_eq!(account.available, "0.0".parse().unwrap());
        assert_eq!(account.held, "0.0".parse().unwrap());
        assert_eq!(account.total, "0.0".parse().unwrap());

        // Already charged back / account locked.
        let tx = super::TransactionType::Chargeback { client: 1, tx: 1 };
        account.process_transaction(tx.clone()).unwrap_err();
        assert_eq!(account.available, "0.0".parse().unwrap());
        assert_eq!(account.held, "0.0".parse().unwrap());
        assert_eq!(account.total, "0.0".parse().unwrap());
    }
}
