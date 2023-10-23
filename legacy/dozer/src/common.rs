use std::sync::Arc;

use zion::prelude::*;
use zion::Zion;

pub struct BytesMessage {
    pub data: Arc<Vec<u8>>,
    pub mesage_id: i64,
}

pub struct CommonPlugin;

impl ZionPlug for CommonPlugin {
    fn deps<'a, 'b>(&mut self, loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader
    }

    fn load<'a>(&mut self, zion: &'a mut Zion) -> &'a mut Zion {
        zion.register_topic::<BytesMessage>()
    }
}
