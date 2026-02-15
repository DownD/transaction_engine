## AI Usage
Github copilot was used to complete some of the unit tests and also the integration test.
Additional it was also used throughout the development for code completions/suggestions.

## Additional assumptions
- Only deposit transactions can be disputed.
- Disputed transactions are only valid if there is enough available funds to cover the disputed amount.
- Negative balances are not possible.
- If an account is frozen, any other subsequent transactions are blocked.

## Testing
Unit tests were made for the transaction engine an verify if the different types of transactions were processed correctly. Also, an integration test was made to verify if the whole process of reading a csv file, processing the transactions and writing the output csv file was working as expected.

## Error handling
Errors during transaction processing like invalid transaction are logged with the `log` crate. 
Any error that occurs on initial setup (parse args or open the file) will cause the program to panic with a message describing the error.

## Performance vs Maintainability
This code prioritizes maintainability over performance as it is preferred in the requirements.
The following list describes some techniques that could be used to improve performance but were not implement due to additional complexity or lack of context for a production environment:

- Multithreading: This could be best achieved by using a thread pool to aggregate transactions in batches and process them in parallel by adding a Mutex to each ClientFunds as well as an RwLock around the client HashMap. To avoid the reordering of transactions, one thread could be responsible for a particular batch of clients. But the overall performance improvement, would probably be bottlenecked by I/O operations.

- MemoryMapping: This would speed up the file loading but would cause a substantial increase in memory usage.

- Use some caching database for transactions: In order to save memory, since most likely disputes should be a much smaller subset of the total transactions, moving them to disk or a caching database could save memory at the cost of some lookup up performance.

## Edge cases

### Memory usage:
As there any previous transaction can be disputed, there is a need to keep track of all transactions in memory, the current solution currently uses a BTreeMap of 10 bytes (might be slightly higher due to alignment) which in turn makes the amount of memory used at least 10 bytes per transaction, this means that for a file with 1 million transactions the memory usage would be at least 10MB just for the transactions, this is not a problem for small datasets but can be a problem for datasets with bilious of transactions.