#![feature(generic_associated_types)]
#![no_std]
use std::prelude::v1::*;

use esl::{ta::rsi::Rsi, *};
// use yata::methods::*;
extern crate no_std_compat as std;

#[strategy]
// My description...
pub fn rsi(
    rsi: Prev<Rsi>,
    // The low line of the rsi.
    lline: Param![1..100, 1],
    // The high line of the rsi.
    hline: Param![1..100, 1],
    mut orders: Orders,
) {
    let lline_condition = rsi.crosses_from_above(lline);
    orders.on(lline_condition, MarketOpenLong::full());
    orders.on(lline_condition, MarketCloseShort::full());
    orders.on(
        lline_condition,
        StopMarketCloseFullLong::relative_to_current_price(0.01),
    );
    let hline_condition = rsi.crosses_from_below(hline);
    orders.on(hline_condition, MarketOpenShort::full());
    orders.on(hline_condition, MarketCloseLong::full());
    orders.on(
        hline_condition,
        StopMarketCloseFullShort::relative_to_current_price(0.01),
    );
}

// #[strategy]
// // My description...
// pub fn sma(sma: Prev<Ind<SMA>>, price: Price, mut orders: Orders) {
//     let buy_condition = sma.crosses_from_below(*price);
//     orders.on(buy_condition, MarketOpenLong::full());
//     orders.on(buy_condition, MarketCloseShort::full());
//     orders.on(
//         buy_condition,
//         StopMarketCloseFullLong::relative_to_current_price(0.01),
//     );
//     let sell_condition = sma.crosses_from_above(*price);
//     orders.on(sell_condition, MarketOpenShort::full());
//     orders.on(sell_condition, MarketCloseLong::full());
//     orders.on(
//         sell_condition,
//         StopMarketCloseFullShort::relative_to_current_price(0.01),
//     );
// }
//
