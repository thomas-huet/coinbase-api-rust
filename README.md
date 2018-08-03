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

### Implemented

- Market Data API (without pagination)
- Private API:
  - GET requests without pagination

### Not implemented yet

- pagination
- Private API:
  - POST requests
  - DELETE requests
  - Payment methods
  - Coinbase accounts
  - Reports
- Websocket Feed
- FIX API
