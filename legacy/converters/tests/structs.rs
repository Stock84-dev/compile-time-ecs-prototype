#![feature(macro_attributes_in_derive_output)]
use std::{
    convert::{From, TryFrom, TryInto},
    error::Error,
    fmt::{Display, Formatter},
};

use converters::{from, into, try_from, try_into};

#[derive(Clone)]
struct CandlesGetRequest {
    timeframe: u32,
    symbol: String,
    count: Option<u32>,
    start_time: Option<u32>,
    end_time: Option<u32>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum BinSize {
    M1,
    M5,
}

impl Default for BinSize {
    fn default() -> Self {
        BinSize::M1
    }
}

#[derive(Debug)]
struct E;

impl Error for E {}

impl Display for E {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unimplemented!()
    }
}

impl TryFrom<u32> for BinSize {
    type Error = E;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            60 => Ok(BinSize::M1),
            300 => Ok(BinSize::M5),
            _ => Err(E),
        }
    }
}

struct A;

#[derive(Clone, Default, Eq, PartialEq, Debug)]
#[try_from(CandlesGetRequest)]
#[from((), skipping)]
#[from(A, constructor = "Default::default()")]
struct GetTradeBucketedRequest {
    #[try_from(CandlesGetRequest, rename = "timeframe")]
    bin_size: BinSize,
    #[try_from(CandlesGetRequest, skip)]
    partial: Option<bool>,
    symbol: String,
    #[try_from(CandlesGetRequest, skip)]
    filter: Option<()>,
    #[try_from(CandlesGetRequest, skip)]
    columns: Option<()>,
    count: Option<u32>,
    #[try_from(CandlesGetRequest, skip)]
    reverse: Option<bool>,
    #[try_from(CandlesGetRequest, with = "other.start_time.map(|x| x as i64)")]
    start: Option<i64>,
    #[try_from(CandlesGetRequest, with = "other.end_time.map(|x| x as i64)")]
    end: Option<i64>,
}

struct Candle {
    timestamp: u32,
    open: f32,
    high: f32,
    low: f32,
    close: f32,
    volume: f32,
}

#[into(Candle, skipping)]
struct TradeBin {
    #[into(with = "self.timestamp as u32")]
    timestamp: i64,
    symbol: String,
    #[into(with = "self.open.unwrap() as f32")]
    open: Option<f64>,
    #[into(with = "self.high.unwrap() as f32")]
    high: Option<f64>,
    #[into(with = "self.low.unwrap() as f32")]
    low: Option<f64>,
    #[into(with = "self.close.unwrap() as f32")]
    close: Option<f64>,
    trades: Option<i64>,
    #[into(with = "self.open.unwrap() as f32")]
    volume: Option<i64>,
    vwap: Option<f64>,
    last_size: Option<i64>,
    turnover: Option<i64>,
    home_notional: Option<f64>,
    foreign_notional: Option<f64>,
}

#[test]
fn try_from_struct() -> Result<(), Box<dyn Error>> {
    let general = CandlesGetRequest {
        timeframe: 60,
        symbol: "ABCDEF".to_string(),
        count: Some(25),
        start_time: Some(1000),
        end_time: None,
    };
    let bucketed = GetTradeBucketedRequest::try_from(general.clone())?;
    assert_eq!(general.symbol, bucketed.symbol);
    assert_eq!(general.start_time.unwrap(), bucketed.start.unwrap() as u32);
    assert!(general.end_time.is_none());
    assert_eq!(general.count, bucketed.count);
    assert_eq!(general.timeframe, 60);
    let bucketed = GetTradeBucketedRequest::from(());
    assert_eq!(bucketed, GetTradeBucketedRequest::default());
    Ok(())
}

#[derive(Clone)]
struct Wrapper(i32);

#[from(Wrapper)]
struct SpecWrapper(i64, #[from(skip)] String, #[from(skip)] ());

#[test]
fn from_tuple_struct() {
    let wrapper = Wrapper(0);
    let spec: SpecWrapper = wrapper.clone().into();
    assert_eq!(spec.0 as i32, wrapper.0);
    assert_eq!(spec.1, String::default());
    assert_eq!(spec.2, ());
    //#[into(with = "self.size.unwrap().into()", rename = "amount")] mutually exclusive when
    // using into
    panic!();
}
