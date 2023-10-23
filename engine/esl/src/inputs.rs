use std::prelude::v1::*;
use crate::*;

pub trait Inputs: 'static {
    fn load<S: Nest, W: World>(nest: &S, world: &mut W);
}

pub trait InputField: 'static {
    type Resource: 'static;
    fn load(&self, resource: &mut Self::Resource);
}

#[input]
pub struct Price(pub f32);

macro_rules! input_layout {
    ($input_nest: ident, $input_layout: ident, $($fields: ident),*) => {
        pub type $input_nest = NestTy!($($fields),*);
        pub type $input_layout = ($($fields),*);
    };
}

pub mod hlcv {
    use crate::*;
    input_layout!(
        HlcvInputNest,
        HlcvInput,
        HighResource,
        LowResource,
        CloseResource,
        VolumeResource
    );
    input_layout!(
        HlcInputNest,
        HlcInput,
        HighResource,
        LowResource,
        CloseResource
    );
    #[input]
    pub struct High(pub f32);
    #[input]
    pub struct Low(pub f32);
    #[resource_value]
    pub struct Close(pub f32);
    impl InputField for CloseResource {
        type Resource = crate::inputs::PriceResource;

        #[inline(always)]
        fn load(&self, resource: &mut Self::Resource) {
            *resource = crate::inputs::PriceResource(self.0);
        }
    }
    #[input]
    pub struct Volume(pub f32);
}

pub mod orderflow {
    use packed_struct::prelude::*;

    use crate::*;
    input_layout!(
        OrderflowInputNest,
        OrderflowInput,
        TimestampNsResource,
        PackedTypeMask,
        PriceResource,
        AmountResource,
        NOrdersResource
    );

    #[input]
    pub struct TimestampNs(pub i64);
    pub struct PackedTypeMask(pub u8);
    impl InputField for PackedTypeMask {
        type Resource = TypeMaskResource;

        #[inline(always)]
        fn load(&self, resource: &mut Self::Resource) {
            *resource = TypeMaskResource::unpack(&[self.0])
                .expect("Failed to unpack `TypeMask` of ordeflow data");
        }
    }
    #[resource_value(skip_value)]
    #[derive(PackedStruct, Debug)]
    #[packed_struct(bit_numbering = "lsb0", size_bytes = "1")]
    /// More docs [here](https://us-futures-market-data-docs.s3.amazonaws.com/algoseek.US.Futures.TAQ.pdf)
    pub struct TypeMask {
        #[packed_field(bits = "0..=4", ty = "enum", size_bits = "5")]
        pub ty: MessageType,
        #[packed_field(bits = "5")]
        /// Final flag (1 if the transaction is complete; 0 if there is another transaction
        /// following this one)
        pub completed: bool,
        #[packed_field(bits = "6")]
        /// Aggressor on the sell-side (trades) or offer (quotes). This is only valid for quotes or
        /// trades
        pub sell: bool,
        #[packed_field(bits = "7")]
        /// Aggressor on the buy-side (trades) or bid (quotes). This is only valid for quotes or
        /// trades
        pub buy: bool,
    }
    pub struct PriceResource(pub f32);
    impl InputField for PriceResource {
        type Resource = crate::inputs::PriceResource;

        #[inline(always)]
        fn load(&self, resource: &mut Self::Resource) {
            *resource = crate::inputs::PriceResource(self.0);
        }
    }
    #[input]
    pub struct Amount(pub f32);
    #[input]
    pub struct NOrders(pub f32);

    #[derive(PrimitiveEnum_u8, Clone, Copy, Debug, PartialEq)]
    pub enum MessageType {
        Heartbeat = 0,
        Quote = 1,
        Trade = 2,
        SessionEnd = 3,
        Prior = 4,
        OpeningPrice = 5,
        ClosingPrice = 6,
        SettlementPrice = 7,
        FixingPrice = 8,
        CashNote = 9,
        TradeVolume = 10,
        OpenInterest = 11,
        EmptyBook = 12,
        AddComponent = 13,
        Update = 14,
        Delete = 15,
        SecurityStatus = 16,
        ElectronicVolume = 17,
        ThresholdLimits = 18,
        BandingHighLimitPriceAdd = 19,
        BandingLowLimitPriceAdd = 20,
        BandingMaxPriceVariationAdd = 21,
        BandingHighLimitPriceRemove = 22,
        BandingLowLimitPriceRemove = 23,
        BandingMaxPriceVariationRemove = 24,
    }

    impl Default for MessageType {
        fn default() -> Self {
            MessageType::Heartbeat
        }
    }
}
