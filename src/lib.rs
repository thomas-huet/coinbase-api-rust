extern crate hyper;
extern crate hyper_tls;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;

use hyper::{Client, Request};
use hyper::rt::{Future, Stream};
use hyper_tls::HttpsConnector;

pub const SANDBOX : &'static str =  "https://api-public.sandbox.pro.coinbase.com";
pub const LIVE : &'static str =  "https://api.pro.coinbase.com";

pub struct PublicClient{
  client : Client<HttpsConnector<hyper::client::HttpConnector>, hyper::Body>,
  base : &'static str,
}

impl PublicClient {
  pub fn new(base : &'static str) -> Result<Self, hyper_tls::Error> {
    let mut https = HttpsConnector::new(4)?;
    https.https_only(true);
    Ok(PublicClient{
      client : Client::builder().build::<_, hyper::Body>(https),
      base
    })
  } 
}

#[derive(Debug)]
pub enum Error {
  HttpError(hyper::error::Error),
  JsonError(serde_json::Error, Result<String, std::string::FromUtf8Error>),
}

#[derive(Serialize, Deserialize)]
pub struct Num(String);

#[derive(Serialize, Deserialize)]
pub struct Product {
  pub id : String,
  pub base_currency : String,
  pub quote_currency : String,
  pub base_min_size : Num,
  pub base_max_size : Num,
  pub quote_increment : Num,
}

impl PublicClient {
  pub fn products(&self) -> impl Future<Item=Vec<Product>, Error=Error> {
    let mut uri = self.base.to_string();
    uri.push_str("/products");

    let req = Request::builder()
      .uri(uri)
      .header(hyper::header::USER_AGENT,"coinbase-api-rust")
      .body(hyper::Body::empty()).unwrap();

    self.client.request(req).and_then(|res| {
      res.into_body().concat2()
    }).map_err(|err| {
      Error::HttpError(err)
    }).and_then(|body| {
      serde_json::from_slice(body.as_ref()).map_err(|err| {
        Error::JsonError(err, String::from_utf8(body.as_ref().to_vec()))
      })
    })
  }
}
