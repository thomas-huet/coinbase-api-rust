// coinbase-api-rust
// Copyright (C) 2018 Thomas HUET
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#![forbid(unsafe_code)]

//! Rust client library for [Coinbase](https://docs.pro.coinbase.com).
//!
//! # Example
//!
//!```
//!extern crate coinbase_api;
//!extern crate hyper;
//!
//!use coinbase_api::*;
//!use hyper::rt::Future;
//!
//!fn make_future() -> impl Future<Item=(), Error=()> {
//!  let client = MarketDataClient::new(SANDBOX).unwrap();
//!  client.products()
//!  .map(|products| {
//!    println!("Pairs available for trading:");
//!    for p in products {
//!      println!("{}", p.id);
//!    }
//!  })
//!  .map_err(|err| println!("Error: {:?}", err))
//!}
//!
//!fn main() {
//!  hyper::rt::run(make_future());
//!}
//!```

extern crate base64;
extern crate chrono;
extern crate hmac;
extern crate hyper;
extern crate hyper_tls;
extern crate serde;
extern crate sha2;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

type DateTime = chrono::DateTime<chrono::Utc>;

/// A decimal number with full precision.
#[derive(Serialize, Deserialize, Debug)]
pub struct Decimal(String);

macro_rules! impl_num_from {
  ($t:ty) => {
    impl From<$t> for Decimal {
      fn from(x : $t) -> Self { Decimal(x.to_string()) }
    }
  };
}

impl_num_from!(f32);
impl_num_from!(f64);
impl_num_from!(i8);
impl_num_from!(i16);
impl_num_from!(i32);
impl_num_from!(i64);
impl_num_from!(isize);
impl_num_from!(u8);
impl_num_from!(u16);
impl_num_from!(u32);
impl_num_from!(u64);
impl_num_from!(usize);

use std::str::FromStr;

impl Decimal {
  pub fn to_f32(&self) -> Option<f32> {
    match self {
      Decimal(s) => match f32::from_str(s) {
        Ok(f) => Some(f),
        Err(_) => None,
      },
    }
  }

  pub fn to_f64(&self) -> Option<f64> {
    match self {
      Decimal(s) => match f64::from_str(s) {
        Ok(f) => Some(f),
        Err(_) => None,
      },
    }
  }
}

/// Description of a currency pair.
#[derive(Deserialize, Debug)]
pub struct Product {
  pub id : String,
  pub base_currency : String,
  pub quote_currency : String,
  pub base_min_size : Decimal,
  pub base_max_size : Decimal,
  pub quote_increment : Decimal,
}

/// Aggregated book of orders.
#[derive(Deserialize, Debug)]
pub struct AggregatedBook {
  pub sequence : u64,
  /// List of (price, size, num-orders).
  pub bids : Vec<(Decimal, Decimal, u64)>,
  pub asks : Vec<(Decimal, Decimal, u64)>,
}

/// Non aggregated book of orders.
#[derive(Deserialize, Debug)]
pub struct FullBook {
  pub sequence : u64,
  /// List of (price, size, order_id).
  pub bids : Vec<(Decimal, Decimal, String)>,
  pub asks : Vec<(Decimal, Decimal, String)>,
}

pub mod book_level {
  use AggregatedBook;
  use FullBook;

  pub struct Best();
  pub struct Top50();
  pub struct Full();

  pub trait BookLevel<T> {
    fn to_str(&self) -> &str;
  }

  impl BookLevel<AggregatedBook> for Best {
    fn to_str(&self) -> &str { "level=1" }
  }

  impl BookLevel<AggregatedBook> for Top50 {
    fn to_str(&self) -> &str { "level=2" }
  }

  impl BookLevel<FullBook> for Full {
    fn to_str(&self) -> &str { "level=3" }
  }
}

/// Information about the last trade (tick), best bid/ask and 24h volume.
#[derive(Deserialize, Debug)]
pub struct Ticker {
  pub trade_id : u64,
  pub price : Decimal,
  pub size : Decimal,
  pub bid : Decimal,
  pub ask : Decimal,
  pub volume : Decimal,
  pub time : DateTime,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Side {
  Buy,
  Sell,
}

/// Description of a trade.
#[derive(Deserialize, Debug)]
pub struct Trade {
  pub time : DateTime,
  pub trade_id : u64,
  pub price : Decimal,
  pub size : Decimal,
  /// Maker order side (buy indicates a down-tick and sell an up-tick).
  pub side : Side,
}

/// `(time, low, high, open, close, volume)`
/// - `time`: Bucket start time.
/// - `low`: Lowest price during the bucket interval.
/// - `high`: Highest price during the bucket interval.
/// - `open`: Opening price (first trade) in the bucket interval.
/// - `close`: Closing price (last trade) in the bucket interval.
/// - `volume`: Volume of trading activity during the bucket interval.
#[derive(Deserialize, Debug)]
pub struct Candle(u64, f64, f64, f64, f64, f64);

/// Trading stats for a product.
/// `volume` is in base currency units. `open`, `high`, `low` are in quote currency units.
#[derive(Deserialize, Debug)]
pub struct Stats {
  pub open : Decimal,
  pub high : Decimal,
  pub low : Decimal,
  pub volume : Decimal,
}

/// Currency description.
#[derive(Deserialize, Debug)]
pub struct Currency {
  pub id : String,
  pub name : String,
  pub min_size : Decimal,
}

/// Time of the API server.
#[derive(Deserialize, Debug)]
pub struct ServerTime {
  pub iso : DateTime,
  pub epoch : f64,
}

/// URL for the sandbox API.
pub const SANDBOX : &str = "https://api-public.sandbox.pro.coinbase.com";

/// URL for the live API.
/// Be sure to test your code on the sandbox API before trying the live one.
pub const LIVE : &str = "https://api.pro.coinbase.com";

use hyper::{
  rt::{Future, Stream},
  Client, Request,
};
use hyper_tls::HttpsConnector;

/// Errors that can happen during a request to the API.
#[derive(Debug)]
pub enum Error {
  HttpError(hyper::error::Error),
  JsonError(
    serde_json::Error,
    Result<String, std::string::FromUtf8Error>,
  ),
}

/// HTTP client for the unauthenticated market data API.
pub struct MarketDataClient {
  client : Client<HttpsConnector<hyper::client::HttpConnector>, hyper::Body>,
  base : &'static str,
}

impl MarketDataClient {
  /// Creates a new client.
  /// The `base` argument should be `SANDBOX` or `LIVE`.
  pub fn new(base : &'static str) -> Result<Self, hyper_tls::Error> {
    let mut https = HttpsConnector::new(4)?;
    https.https_only(true);
    Ok(MarketDataClient {
      client : Client::builder().build::<_, hyper::Body>(https),
      base,
    })
  }

  fn get<T>(&self, uri : &str) -> impl Future<Item = T, Error = Error>
  where
    T : serde::de::DeserializeOwned,
  {
    let req = Request::builder()
      .uri(uri)
      .header(hyper::header::USER_AGENT, "coinbase-api-rust")
      .body(hyper::Body::empty())
      .unwrap();
    self
      .client
      .request(req)
      .and_then(|res| res.into_body().concat2())
      .map_err(Error::HttpError)
      .and_then(|body| {
        serde_json::from_slice(body.as_ref())
          .map_err(|err| Error::JsonError(err, String::from_utf8(body.as_ref().to_vec())))
      })
  }

  /// Retrieves a list of available currency pairs for trading.
  pub fn products(&self) -> impl Future<Item = Vec<Product>, Error = Error> {
    let mut uri = self.base.to_string();
    uri.push_str("/products");
    self.get(&uri)
  }

  /// Retrieves a list of open orders for a product.
  /// The amount of detail shown depends on the level argument:
  /// - `book_level::Best()` shows only the best bid and ask.
  /// - `book_level::Top50()` shows the top 50 aggregated bids and asks.
  /// - `book_level::Full()` shows the full non aggregated order book.
  pub fn book<T>(
    &self,
    product_id : &str,
    level : &book_level::BookLevel<T>,
  ) -> impl Future<Item = T, Error = Error>
  where
    T : serde::de::DeserializeOwned,
  {
    let mut uri = self.base.to_string();
    uri.push_str("/products/");
    uri.push_str(product_id);
    uri.push_str("/book?");
    uri.push_str(level.to_str());
    self.get(&uri)
  }

  /// Retrieves information about the last trade (tick), best bid/ask and 24h volume.
  pub fn ticker(&self, product_id : &str) -> impl Future<Item = Ticker, Error = Error> {
    let mut uri = self.base.to_string();
    uri.push_str("/products/");
    uri.push_str(product_id);
    uri.push_str("/ticker");
    self.get(&uri)
  }

  /// Lists the latest trades for a product.
  pub fn trades(&self, product_id : &str) -> impl Future<Item = Vec<Trade>, Error = Error> {
    let mut uri = self.base.to_string();
    uri.push_str("/products/");
    uri.push_str(product_id);
    uri.push_str("/trades");
    self.get(&uri)
  }

  /// Retrieves historic rates for a product.
  /// `granularity` must be one of { one minute, five minutes, fifteen minutes, one hour, six hours, one day }.
  /// The maximum number of data points for a single request is 300 candles.
  /// If your selection of start/end time and granularity will result in more than 300 data points, your request will be rejected.
  pub fn candles(
    &self,
    product_id : &str,
    start : &DateTime,
    end : &DateTime,
    granularity : &chrono::Duration,
  ) -> impl Future<Item = Vec<Candle>, Error = Error> {
    let mut uri = self.base.to_string();
    uri.push_str("/products/");
    uri.push_str(product_id);
    uri.push_str("/candles?start=");
    uri.push_str(&start.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    uri.push_str("&end=");
    uri.push_str(&end.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
    uri.push_str("&granularity=");
    uri.push_str(&granularity.num_seconds().to_string());
    self.get(&uri)
  }

  /// Retrieves the latest 300 data points.
  pub fn latest_candles(
    &self,
    product_id : &str,
    granularity : &chrono::Duration,
  ) -> impl Future<Item = Vec<Candle>, Error = Error> {
    let mut uri = self.base.to_string();
    uri.push_str("/products/");
    uri.push_str(product_id);
    uri.push_str("/candles?granularity=");
    uri.push_str(&granularity.num_seconds().to_string());
    self.get(&uri)
  }

  /// Retrieves 24 hr stats for the product.
  pub fn stats(&self, product_id : &str) -> impl Future<Item = Stats, Error = Error> {
    let mut uri = self.base.to_string();
    uri.push_str("/products/");
    uri.push_str(product_id);
    uri.push_str("/stats");
    self.get(&uri)
  }

  /// Lists known currencies.
  pub fn currencies(&self) -> impl Future<Item = Vec<Currency>, Error = Error> {
    let mut uri = self.base.to_string();
    uri.push_str("/currencies");
    self.get(&uri)
  }

  /// Gets the API server time.
  pub fn time(&self) -> impl Future<Item = ServerTime, Error = Error> {
    let mut uri = self.base.to_string();
    uri.push_str("/time");
    self.get(&uri)
  }
}

/// Description of a trading account.
#[derive(Deserialize, Debug)]
pub struct Account {
  pub id : String,
  pub currency : String,
  pub balance : Decimal,
  pub available : Decimal,
  pub hold : Decimal,
  pub profile_id : String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ActivityType {
  Transfer,
  Match,
  Fee,
  Rebate,
}

#[derive(Deserialize, Debug)]
pub struct ActivityDetails {
  pub order_id : Option<String>,
  pub trade_id : Option<String>,
  pub product_id : Option<String>,
  pub transfer_id : Option<String>,
  pub transfer_type : Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Activity {
  pub id : u64,
  pub created_at : DateTime,
  pub amount : Decimal,
  pub balance : Decimal,
  #[serde(rename = "type")]
  pub activity_type : ActivityType,
  pub details : ActivityDetails,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum HoldType {
  Order,
  Transfer,
}

#[derive(Deserialize, Debug)]
pub struct Hold {
  pub id : String,
  pub created_at : DateTime,
  pub updated_at : Option<DateTime>,
  pub amount : Decimal,
  #[serde(rename = "type")]
  pub hold_type : HoldType,
  #[serde(rename = "ref")]
  pub hold_ref : String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
  Limit,
  Market,
}

#[derive(Deserialize, Debug)]
pub struct Order {
  pub id : String,
  pub price : Option<Decimal>,
  pub size : Option<Decimal>,
  pub product_id : String,
  pub side : Side,
  pub stp : Option<String>,
  pub funds : Option<Decimal>,
  pub specified_funds : Option<Decimal>,
  #[serde(rename = "type")]
  pub order_type : OrderType,
  pub time_in_force : Option<String>,
  pub post_only : bool,
  pub created_at : DateTime,
  pub done_at : Option<DateTime>,
  pub done_reason : Option<String>,
  pub fill_fees : Decimal,
  pub filled_size : Decimal,
  pub executed_value : Decimal,
  pub status : String,
  pub settled : bool,
}

#[derive(Deserialize, Debug)]
pub struct Fill {
  pub trade_id : u64,
  pub product_id : String,
  pub price : Decimal,
  pub size : Decimal,
  pub order_id : String,
  pub created_at : DateTime,
  pub liquidity : String,
  pub fee : Decimal,
  pub settled : bool,
  pub side : Side,
}

#[derive(Deserialize, Debug)]
pub struct TrailingVolume {
  pub product_id : String,
  pub exchange_volume : Decimal,
  pub volume : Decimal,
  pub recorded_at : DateTime,
}

fn now() -> u64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_secs()
}

/// HTTP client for the authenticated private API.
pub struct PrivateClient {
  client : Client<HttpsConnector<hyper::client::HttpConnector>, hyper::Body>,
  base : &'static str,
  key : String,
  secret : String,
  passphrase : String,
}

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

impl PrivateClient {
  /// Creates a new client.
  /// The `base` argument should be `SANDBOX` or `LIVE`.
  pub fn new(
    base : &'static str,
    key : String,
    secret : String,
    passphrase : String,
  ) -> Result<Self, hyper_tls::Error> {
    let mut https = HttpsConnector::new(4)?;
    https.https_only(true);
    Ok(PrivateClient {
      client : Client::builder().build::<_, hyper::Body>(https),
      base,
      key,
      secret,
      passphrase,
    })
  }

  fn sign(&self, timestamp : u64, method : &hyper::Method, path : &str, body : &str) -> String {
    let mut message = timestamp.to_string();
    message.push_str(method.as_str());
    message.push_str(path);
    message.push_str(body);
    let mut mac = HmacSha256::new_varkey(&base64::decode(&self.secret).unwrap()).unwrap();
    mac.input(message.as_bytes());
    base64::encode(&mac.result().code())
  }

  fn get<T>(&self, query : &str) -> impl Future<Item = T, Error = Error>
  where
    T : serde::de::DeserializeOwned,
  {
    let mut uri = self.base.to_string();
    uri.push_str(query);
    let timestamp = now();
    let req = Request::builder()
      .uri(uri)
      .header(hyper::header::USER_AGENT, "coinbase-api-rust")
      .header("cb-access-key", self.key.as_str())
      .header("cb-access-passphrase", self.passphrase.as_str())
      .header("cb-access-timestamp", timestamp)
      .header(
        "cb-access-sign",
        self
          .sign(timestamp, &hyper::Method::GET, query, "")
          .as_str(),
      ).body(hyper::Body::empty())
      .unwrap();
    self
      .client
      .request(req)
      .and_then(|res| res.into_body().concat2())
      .map_err(Error::HttpError)
      .and_then(|body| {
        serde_json::from_slice(body.as_ref())
          .map_err(|err| Error::JsonError(err, String::from_utf8(body.as_ref().to_vec())))
      })
  }

  pub fn accounts(&self) -> impl Future<Item = Vec<Account>, Error = Error> {
    self.get("/accounts")
  }

  pub fn account(&self, id : &str) -> impl Future<Item = Account, Error = Error> {
    let mut query = "/accounts/".to_string();
    query.push_str(id);
    self.get(&query)
  }

  pub fn ledger(&self, id : &str) -> impl Future<Item = Vec<Activity>, Error = Error> {
    let mut query = "/accounts/".to_string();
    query.push_str(id);
    query.push_str("/ledger");
    self.get(&query)
  }

  pub fn holds(&self, id : &str) -> impl Future<Item = Vec<Hold>, Error = Error> {
    let mut query = "/accounts/".to_string();
    query.push_str(id);
    query.push_str("/holds");
    self.get(&query)
  }

  pub fn orders(&self) -> impl Future<Item = Vec<Order>, Error = Error> {
    self.get("/orders?status=all")
  }

  pub fn orders_for_product(&self, id : &str) -> impl Future<Item = Vec<Order>, Error = Error> {
    let mut query = "/orders?status=all&product_id=".to_string();
    query.push_str(id);
    self.get(&query)
  }

  pub fn order(&self, id : &str) -> impl Future<Item = Order, Error = Error> {
    let mut query = "/orders/".to_string();
    query.push_str(id);
    self.get(&query)
  }

  pub fn fills(&self) -> impl Future<Item = Vec<Fill>, Error = Error> { self.get("/fills") }

  pub fn fills_for_product(&self, id : &str) -> impl Future<Item = Vec<Fill>, Error = Error> {
    let mut query = "/fills?product_id=".to_string();
    query.push_str(id);
    self.get(&query)
  }

  pub fn fills_for_order(&self, id : &str) -> impl Future<Item = Vec<Fill>, Error = Error> {
    let mut query = "/fills?order_id=".to_string();
    query.push_str(id);
    self.get(&query)
  }

  pub fn trailing_volume(&self) -> impl Future<Item = Vec<TrailingVolume>, Error = Error> {
    self.get("/users/self/trailing-volume")
  }
}
