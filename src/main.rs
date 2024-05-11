use std::str::FromStr;

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

#[derive(Debug, Serialize, Deserialize)]
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
    }

    Ok(())
}
