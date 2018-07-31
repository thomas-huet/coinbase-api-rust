# Coinbase-api

Client library for the Coinbase Pro API.
Requests are asynchronous and return futures.

[Coinbase API reference](https://docs.pro.coinbase.com)

## Example

Basic example to list all currency pairs trading on the exchange:

```rust
extern crate coinbase_api;
extern crate hyper;

use coinbase_api::*;
use hyper::rt::Future;

fn make_future() -> impl Future<Item=(), Error=()> {
  let client = MarketDataClient::new(SANDBOX).unwrap();
  client.products()
  .map(|products| {
    println!("Pairs available for trading:");
    for p in products {
      println!("{}", p.id);
    }
  })
  .map_err(|err| println!("Error: {:?}", err))
}

fn main() {
  hyper::rt::run(make_future());
}
```

## Progress

### Working

- Market Data API:
  - get products
  - get product order book
  - get product ticker
  - get trades (not paginated)

### Not implemented yet

- Market Data API:
  - historic rates
  - 24h stats
  - currencies
  - time
  - pagination
- Private API
- Websocket Feed
- FIX API
