use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

pub const PRECISION: u32 = 4;

/// Represents a transaction in the CSV file.
#[derive(Debug, Serialize, Deserialize)]
pub struct CsvTransaction {
    #[serde(rename = "type")]
    ty: String,
    client: u16,
    tx: u32,
    amount: Option<String>,
}

/// Represents the type of transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    /// A deposit transaction.
    Deposit {
        client: u16,
        tx: u32,
        amount: Decimal,
    },
    /// A withdrawal transaction.
    Withdrawal {
        client: u16,
        tx: u32,
        amount: Decimal,
    },
    /// A dispute transaction.
    Dispute { client: u16, tx: u32 },
    /// A resolve transaction.
    Resolve { client: u16, tx: u32 },
    /// A chargeback transaction.
    Chargeback { client: u16, tx: u32 },
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
