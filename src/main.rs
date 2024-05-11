mod client;
mod engine;
mod types;

use crate::{
    engine::PaymentEngine,
    types::{CsvTransaction, TransactionType},
};

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
