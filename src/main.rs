use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct CsvTransaction {
    #[serde(rename = "type")]
    ty: String,
    client: u16,
    tx: u32,
    amount: f64,
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
        let Ok(record) = line.map_err(|err| {
            log::error!("Unprocessed line {err:?}");
            err
        }) else {
            continue;
        };
        let record: CsvTransaction = record;

        log::trace!("{:?}", record);
    }

    Ok(())
}
