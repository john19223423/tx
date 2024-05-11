use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct CsvTransaction {
    #[serde(rename = "type")]
    ty: String,
    client: u16,
    tx: u32,
    amount: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum TransactionType {
    Deposit {
        client: u16,
        tx: u32,
        amount: Decimal,
    },
    Withdrawal {
        client: u16,
        tx: u32,
        amount: Decimal,
    },
    Dispute {
        client: u16,
        tx: u32,
    },
    Resolve {
        client: u16,
        tx: u32,
    },
    Chargeback {
        client: u16,
        tx: u32,
    },
}

impl TransactionType {
    /// Returns the client ID associated with the transaction.
    pub fn client_id(&self) -> u16 {
        match self {
            Self::Deposit { client, .. } => *client,
            Self::Withdrawal { client, .. } => *client,
            Self::Dispute { client, .. } => *client,
            Self::Resolve { client, .. } => *client,
            Self::Chargeback { client, .. } => *client,
        }
    }

    /// Returns the transaction ID associated with the transaction.
    pub fn transaction_id(&self) -> u32 {
        match self {
            Self::Deposit { tx, .. } => *tx,
            Self::Withdrawal { tx, .. } => *tx,
            Self::Dispute { tx, .. } => *tx,
            Self::Resolve { tx, .. } => *tx,
            Self::Chargeback { tx, .. } => *tx,
        }
    }
}

const PRECISION: u32 = 4;

impl TryFrom<CsvTransaction> for TransactionType {
    type Error = &'static str;

    fn try_from(value: CsvTransaction) -> Result<Self, Self::Error> {
        // Small helper to ensure we always have the required precision.
        let parse_decimal = |value: String| -> Result<Decimal, Self::Error> {
            let dec = Decimal::from_str(&value).map_err(|_| "invalid decimal")?;
            if dec.scale() > PRECISION {
                return Err("Invalid precision");
            }
            Ok(dec)
        };

        match value.ty.as_str() {
            "deposit" => Ok(Self::Deposit {
                client: value.client,
                tx: value.tx,
                amount: parse_decimal(value.amount.ok_or("No amount provided")?)?,
            }),
            "withdrawal" => Ok(Self::Withdrawal {
                client: value.client,
                tx: value.tx,
                amount: parse_decimal(value.amount.ok_or("No amount provided")?)?,
            }),
            "dispute" => Ok(Self::Dispute {
                client: value.client,
                tx: value.tx,
            }),
            "resolve" => Ok(Self::Resolve {
                client: value.client,
                tx: value.tx,
            }),
            "chargeback" => Ok(Self::Chargeback {
                client: value.client,
                tx: value.tx,
            }),
            _ => Err("Unknown transaction type"),
        }
    }
}

#[derive(Debug)]
enum ClientErr {
    AccountLocked,
    InsufficientFunds,
    DisputedTransactionNotFound,
    AlreadyProcessed,
}

#[derive(Debug, Default)]
struct ClientAccount {
    client: u16,
    available: Decimal,
    held: Decimal,
    total: Decimal,
    locked: bool,

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

struct PaymentEngine {
    accounts: HashMap<u16, ClientAccount>,
}

impl PaymentEngine {
    /// Constructs a new [`PaymentEngine`].
    fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    /// Process the given transaction.
    fn process_transaction(&mut self, tx: TransactionType) {
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
    fn serialize(&self) -> Result<(), Box<dyn std::error::Error>> {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let Some(file_location) = std::env::args().nth(1) else {
        return Err("Please provide a file location".into());
    };
    log::debug!("File location: {}", file_location);

    let file = std::fs::File::open(file_location)?;

    let mut csv_reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .has_headers(true)
        .from_reader(file);

    let mut engine = PaymentEngine::new();

    for line in csv_reader.deserialize() {
        let record: CsvTransaction = match line {
            Ok(record) => record,
            Err(err) => {
                log::error!("Unprocessed line {err:?}");
                continue;
            }
        };

        let tx = match TransactionType::try_from(record) {
            Ok(tx) => tx,
            Err(err) => {
                log::error!("Error processing transaction: {err}");
                continue;
            }
        };
        log::trace!("{:?}", tx);

        engine.process_transaction(tx);
    }

    engine.serialize()?;

    Ok(())
}
