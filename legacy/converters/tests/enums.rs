#![feature(macro_attributes_in_derive_output)]
use converters::{from, into, try_from, try_into};

#[derive(Clone)]
enum Message {
    Msg(String),
    Empty(()),
    Other,
}

#[from(Message, default = "SpecMessage::Other")]
enum SpecMessage {
    #[from(rename = "Msg", with = "SpecMessage::Message(string_into_i32(arg0))")]
    Message(i32),
    #[from(skip)]
    MessageString(String),
    Other,
}

fn string_into_i32(value: String) -> i32 {
    match value.as_str() {
        "0" => 0,
        "1" => 1,
        _ => unreachable!(),
    }
}

#[test]
fn from_enum() {
    let message = Message::Msg("0".into());
    let spec: SpecMessage = message.into();
    match spec {
        SpecMessage::Message(i) => assert_eq!(i, 0),
        _ => unreachable!(),
    }

    let message = Message::Empty(());
    let spec: SpecMessage = message.into();
    match spec {
        SpecMessage::Other => {},
        _ => unreachable!(),
    }
}

enum V {
    VariantA(i32),
    VariantB { x: u8, y: u16 },
}

#[test]
fn a() {
    let v: V = V::VariantA(111);
    match v {
        V::VariantA { 0: arg0 } => V::VariantA { 0: arg0 },
        V::VariantB { x, y } => V::VariantB { x, y },
    };
}
