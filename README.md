## Transaction Engine

The core of the transaction engine is implemented by the `PaymentEngine` object.
The transaction engine supports the following transaction types:
- Deposit
- Withdrawal
- Dispute
- Resolve
- Chargeback

### Modules
- _types.rs_ This module contains the transaction type that is handed to the payment engine to process, as well as the raw CSV transaction record that is expected to be read from the input file.
- _engine.rs_ This module contains the `PaymentEngine` object that processes the transactions. The transactions are forwarded to the `ClientAccount` object to be processed. When the transaction identifies an account that has not been seen before, a new `ClientAccount` object is created and stored in the `PaymentEngine` object. The `PaymentEngine` object is responsible for maintaining the state of the accounts and produces a serialized CSV output at the end of the processing.
- _client.rs_ This module contains the `ClientAccount` object that represents the state of an account. It contains the account number, the balance and the list of transactions that have been processed. Special consideration was taken to facilitate the processing of disputes, resolves and chargebacks. The `ClientAccount` object is responsible for processing the transactions and maintaining the state of the account
  - A locked account cannot process transactions.
  - An account becomes locked when a chargeback transaction is successfully processed.
  - A withdraw transaction is only processed if the account has sufficient funds.
  - A dispute transaction is only implemented for deposits
  - Resolving a dispute requires the transaction id to be marked as disputed. In other words, the transaction id must be present in the disputed transactions list. Similar for the chargeback transaction.
  - The disputed transactions list is populated when a dispute transaction is processed. The disputed transactions list is cleared when a chargeback transaction is processed. Similar for the chargeback transaction. This is to ensure that the disputed transactions are only resolved or chargebacked once.
  - Floating point precision is handled by using the `Decimal` type from the `rust_decimal` crate. This is to ensure that the balance is maintained accurately.

### Testing
- Various tests are implemented to ensure that the transaction engine works as expected. The tests are located under `client.rs` and `engine.rs`:
  - Check deposits and withdraws with positive and negative amounts
  - Reproduce replay attacks by resubmitting the same transaction again
  - Dispute resolutions via the `resolve` transaction
  - Dispute resolutions via the `chargeback` transaction
  - A locked account cannot be processed
- Various manually created csv files to check the correctness of the engine. The csv files are located under the `artifacts` directory.

### Extensions and Future Considerations
- The transaction engine operates in a single-threaded manner. To ensure the `PaymentEngine` is thread-safe (Send + Sync), the `PaymentEngine` object can be wrapped in a `Arc<Mutex<..>>` object. 

- At the moment the logic of the `ClientAccount` and the number of transactions that is supports are limited by the memory of the machine (this is an in memory implementation). To support a large number of transactions, the `ClientAccount` object can be stored in a database / on disk. Then, only a subset of the accounts can be loaded into memory at a time. This can be achieved by implementing a `Database` object that can store and retrieve the `ClientAccount` object.