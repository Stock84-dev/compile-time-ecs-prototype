use crate::types::{Direction, OrderId, PositionAction, PositionSize};

#[derive(Clone, Debug, PartialEq)]
pub struct OrderCreated {
    pub id: OrderId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrderExecuted {
    pub id: OrderId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrderCanceled {
    pub id: OrderId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PositionUpdated {
    pub position_action: PositionAction,
    pub direction: Direction,
    pub size: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PositionOpened {
    pub direction: Direction,
    pub size: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PositionClosed {
    pub direction: Direction,
    pub size: f32,
}

#[non_exhaustive]
/// Not for public use. Use specific orders from this module.
#[derive(Clone, Debug, PartialEq)]
pub enum OrderPlaced {
    // Delibarately not using structs so that `strategy` macro could parse function body.
    Market {
        size: PositionSize,
        position_action: PositionAction,
        direction: Direction,
    },
    StopMarket {
        size: PositionSize,
        position_action: PositionAction,
        direction: Direction,
        trigger: f32,
    },
}
