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

extern crate hyper;
extern crate hyper_tls;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use hyper::{
  rt::{Future, Stream},
  Client, Request,
};
use hyper_tls::HttpsConnector;

pub const SANDBOX : &str = "https://api-public.sandbox.pro.coinbase.com";
pub const LIVE : &str = "https://api.pro.coinbase.com";

pub struct PublicClient {
  client : Client<HttpsConnector<hyper::client::HttpConnector>, hyper::Body>,
  base : &'static str,
}

impl PublicClient {
  pub fn new(base : &'static str) -> Result<Self, hyper_tls::Error> {
    let mut https = HttpsConnector::new(4)?;
    https.https_only(true);
    Ok(PublicClient {
      client : Client::builder().build::<_, hyper::Body>(https),
      base,
    })
  }

  fn get<T>(&self, uri : String) -> impl Future<Item = T, Error = Error>
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
      .map_err(|err| Error::HttpError(err))
      .and_then(|body| {
        serde_json::from_slice(body.as_ref())
          .map_err(|err| Error::JsonError(err, String::from_utf8(body.as_ref().to_vec())))
      })
  }
}

#[derive(Debug)]
pub enum Error {
  HttpError(hyper::error::Error),
  JsonError(
    serde_json::Error,
    Result<String, std::string::FromUtf8Error>,
  ),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Num(String);

#[derive(Serialize, Deserialize, Debug)]
pub struct Product {
  pub id : String,
  pub base_currency : String,
  pub quote_currency : String,
  pub base_min_size : Num,
  pub base_max_size : Num,
  pub quote_increment : Num,
}

impl PublicClient {
  pub fn products(&self) -> impl Future<Item = Vec<Product>, Error = Error> {
    let mut uri = self.base.to_string();
    uri.push_str("/products");

    self.get(uri)
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AggregatedBook {
  pub sequence : u64,
  pub bids : Vec<(Num, Num, u64)>,
  pub asks : Vec<(Num, Num, u64)>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FullBook {
  pub sequence : u64,
  pub bids : Vec<(Num, Num, String)>,
  pub asks : Vec<(Num, Num, String)>,
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

impl PublicClient {
  pub fn book<T>(
    &self,
    id : &str,
    level : &book_level::BookLevel<T>,
  ) -> impl Future<Item = T, Error = Error>
  where
    T : serde::de::DeserializeOwned,
  {
    let mut uri = self.base.to_string();
    uri.push_str("/products/");
    uri.push_str(id);
    uri.push_str("/book?");
    uri.push_str(level.to_str());

    self.get(uri)
  }
}
