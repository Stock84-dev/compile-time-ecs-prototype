use crate::{
    events::OrderPlaced,
    types::{Direction, PositionAction, PositionSize, TriggerPrice},
};

pub struct StopMarketOpenFullShort {
    pub trigger: TriggerPrice,
}

pub struct StopMarketOpenFullLong {
    pub trigger: TriggerPrice,
}

pub struct StopMarketCloseFullShort {
    pub trigger: TriggerPrice,
}

pub struct StopMarketCloseFullLong {
    pub trigger: TriggerPrice,
}

pub struct StopMarketOpenShort {
    pub size: PositionSize,
    pub trigger: f32,
}

pub struct StopMarketOpenLong {
    pub size: PositionSize,
    pub trigger: f32,
}

pub struct StopMarketCloseShort {
    pub size: PositionSize,
    pub trigger: f32,
}

pub struct StopMarketCloseLong {
    pub size: PositionSize,
    pub trigger: f32,
}

pub struct MarketOpenShort {
    pub size: PositionSize,
}

pub struct MarketOpenLong {
    pub size: PositionSize,
}

pub struct MarketCloseShort {
    pub size: PositionSize,
}

pub struct MarketCloseLong {
    pub size: PositionSize,
}

pub trait IntoOrder {
    fn into_order(self, entry_price: f32) -> OrderPlaced;
}

impl<T: Into<OrderPlaced>> IntoOrder for T {
    #[inline(always)]
    fn into_order(self, _entry_price: f32) -> OrderPlaced {
        self.into()
    }
}

macro_rules! impl_from_stop_market_order {
    ($order:ty, $action:expr, $direction:expr) => {
        impl IntoOrder for $order {
            #[inline(always)]
            fn into_order(self, current_price: f32) -> OrderPlaced {
                OrderPlaced::StopMarket {
                    trigger: match self.trigger {
                        TriggerPrice::Absolute(trigger) => trigger,
                        TriggerPrice::RelativeToCurrentPrice(trigger) => match $direction {
                            Direction::Long => current_price * (1. - trigger),
                            Direction::Short => current_price * (1. + trigger),
                        },
                    },
                    size: PositionSize::Relative(1.),
                    position_action: $action,
                    direction: $direction,
                }
            }
        }
    };
}

impl_from_stop_market_order!(
    StopMarketOpenFullShort,
    PositionAction::Open,
    Direction::Short
);
impl_from_stop_market_order!(
    StopMarketOpenFullLong,
    PositionAction::Open,
    Direction::Long
);
impl_from_stop_market_order!(
    StopMarketCloseFullShort,
    PositionAction::Close,
    Direction::Short
);
impl_from_stop_market_order!(
    StopMarketCloseFullLong,
    PositionAction::Close,
    Direction::Long
);

macro_rules! impl_from_market_order {
    ($order:ty, $action:expr, $direction:expr) => {
        impl From<$order> for OrderPlaced {
            #[inline(always)]
            fn from(value: $order) -> Self {
                OrderPlaced::Market {
                    size: value.size,
                    position_action: $action,
                    direction: $direction,
                }
            }
        }
    };
}

impl_from_market_order!(MarketOpenShort, PositionAction::Open, Direction::Short);
impl_from_market_order!(MarketOpenLong, PositionAction::Open, Direction::Long);
impl_from_market_order!(MarketCloseShort, PositionAction::Close, Direction::Short);
impl_from_market_order!(MarketCloseLong, PositionAction::Close, Direction::Long);

macro_rules! impl_stop_order_full {
    ($order:ty) => {
        impl $order {
            // No relative to avoid confusion of which reference
            #[inline(always)]
            pub fn absolute(trigger: f32) -> Self {
                Self {
                    trigger: TriggerPrice::Absolute(trigger),
                }
            }

            #[inline(always)]
            pub fn relative_to_current_price(trigger: f32) -> Self {
                Self {
                    trigger: TriggerPrice::RelativeToCurrentPrice(trigger),
                }
            }
        }
    };
}

impl_stop_order_full!(StopMarketOpenFullLong);
impl_stop_order_full!(StopMarketOpenFullShort);
impl_stop_order_full!(StopMarketCloseFullLong);
impl_stop_order_full!(StopMarketCloseFullShort);

macro_rules! impl_order {
    ($order:ty) => {
        impl $order {
            #[inline(always)]
            pub fn full() -> Self {
                Self {
                    size: PositionSize::Relative(1.),
                }
            }

            #[inline(always)]
            pub fn relative(size: f32) -> Self {
                Self {
                    size: PositionSize::Relative(size),
                }
            }

            #[inline(always)]
            pub fn absolute(size: f32) -> Self {
                Self {
                    size: PositionSize::Absolute(size),
                }
            }
        }
    };
}

impl_order!(MarketOpenLong);
impl_order!(MarketOpenShort);
impl_order!(MarketCloseLong);
impl_order!(MarketCloseShort);
