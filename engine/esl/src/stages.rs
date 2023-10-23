use inception::{stages::*, *};

pub type Init = Stage0;
pub type BacktestInit = Stage1;
pub type CatchUp = Stage2;
pub type UpdatePrev = Stage3;
pub type IncPreLoopIndex = Stage4;
pub type Input0 = Stage5;
pub type Input1 = Stage6;
pub type IndicatorCompute = Stage7;
pub type Signal = Stage8;
pub type Trade = Stage9;
pub type PostTrade0 = Stage10;
pub type PostTrade1 = Stage11;
pub type PostTrade2 = Stage12;
pub type PostTrade3 = Stage13;
pub type PostTrade4 = Stage14;
pub type PostTrade5 = Stage15;
pub type PostTrade6 = Stage16;
pub type PostTrade7 = Stage17;
pub type PostTrade8 = Stage18;
pub type PostBlock0 = Stage19;
pub type PostBlock1 = Stage20;
pub type PostBlock2 = Stage21;
pub type PostBlock3 = Stage22;
pub type PostBlock4 = Stage23;
pub type PostBlock5 = Stage24;
pub type PostBlock6 = Stage25;
pub type PostBlock7 = Stage26;
pub type PostBlock8 = Stage27;
pub type IncLoopIndex = Stage29;
pub type End = Stage30;
pub use inception::stages::Last;

schedule! {
    struct BacktestSchedule,
    Stage0 as Init,
    Stage1 as BacktestInit,
    Stage2 as CatchUp,
    Stage3 as UpdatePrev,
    Stage4 as IncPreLoopIndex,
    loop {
        Stage5 as Input0,
        Stage6 as Input1,
        Stage7 as IndicatorCompute,
        Stage8 as Signal,
        Stage9 as Trade,
        Stage10 as PostTrade0,
        Stage11 as PostTrade1,
        Stage12 as PostTrade2,
        Stage13 as PostTrade3,
        Stage14 as PostTrade4,
        Stage15 as PostTrade5,
        Stage16 as PostTrade6,
        Stage17 as PostTrade7,
        Stage18 as PostTrade8,
        Stage29 as IncLoopIndex,
        Stage31 as Last,
    },
    Stage19 as PostBlock0,
    Stage20 as PostBlock1,
    Stage21 as PostBlock2,
    Stage22 as PostBlock3,
    Stage23 as PostBlock4,
    Stage24 as PostBlock5,
    Stage25 as PostBlock6,
    Stage26 as PostBlock7,
    Stage27 as PostBlock8,
    Stage30 as End,
}
