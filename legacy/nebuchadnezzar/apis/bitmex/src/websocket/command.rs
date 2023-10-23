use nebuchadnezzar_core::{
    commands::PingSuperCommand,
    prelude::{Deserialize, Serialize},
    serde_json,
    websocket::{
        tokio_tungstenite::tungstenite::Message as RawMessage, CommandConverter, WebSocket,
        WsCommand,
    },
};

use super::Topic;
use crate::websocket::BitmexWebSocket;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", content = "args")]
#[serde(rename_all = "camelCase")]
pub enum Command {
    Subscribe(Vec<Topic>),
    Unsubscribe(Vec<Topic>),
    #[serde(rename = "authKeyExpires")]
    Authenticate(String, i64, String), // ApiKey, Expires, Signature
    CancelAllAfter(i64),
    Ping,
}

impl WsCommand<BitmexWebSocket> for Command {
    fn serialize(&self) -> <BitmexWebSocket as WebSocket>::RawCommand {
        match self {
            Command::Ping => RawMessage::Ping(Vec::new()),
            _ => RawMessage::Text(serde_json::to_string(self).unwrap()),
        }
    }
}

impl_command_converter! {
    for BitmexWebSocket;
    PingSuperCommand => Command
}

impl From<PingSuperCommand> for Command {
    fn from(_: PingSuperCommand) -> Self {
        Command::Ping
    }
}

#[cfg(test)]
mod t_command {
    use nebuchadnezzar_core::{
        error::AnyResult,
        websocket::{RawMessage, WsCommand},
    };

    use crate::websocket::Command;

    #[test]
    fn t_serialize_command() -> AnyResult<()> {
        let command = Command::Ping;
        let serialized = command.serialize()?;
        assert_eq!(serialized, RawMessage::Ping(Vec::new()));
        Ok(())
    }
}
