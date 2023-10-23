use std::prelude::v1::*;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PositionAction {
    Open,
    Close,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Long,
    Short,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PositionSize {
    Relative(f32),
    Absolute(f32),
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Slippage {
    Relative(f32),
    Absolute(f32),
}

impl Default for Slippage {
    fn default() -> Self {
        Slippage::Absolute(0.0)
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fee {
    RelativeToVolume(f32),
}

impl Default for Fee {
    fn default() -> Self {
        Fee::RelativeToVolume(0.0)
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TriggerPrice {
    RelativeToCurrentPrice(f32),
    Absolute(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrderId(pub u32);

