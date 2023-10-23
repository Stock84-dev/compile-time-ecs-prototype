use std::sync::Arc;

use bevy::ecs::schedule::SystemDescriptor;
use zion::prelude::*;
use zion::Zion;

pub struct HlcvPlugin;
impl ZionPlug for HlcvPlugin {
    fn deps<'a, 'b>(&mut self, loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader
    }

    fn load<'a>(&mut self, zion: &'a mut Zion) -> &'a mut Zion {
        zion
    }
}

pub struct HlcvWriter {}

#[async_trait]
impl Pipe for HlcvWriter {
    fn layout() -> PipeLayout
    where
        Self: Sized,
    {
        todo!()
    }

    fn new<'a>(builder: &mut ParamBuilder<'a>) -> AnyResult<Arc<dyn Pipe>>
    where
        Self: Sized,
    {
        todo!()
    }

    async fn spawn(self: Arc<Self>) -> AnyResult<()> {
        todo!()
    }

    fn system(&self) -> Option<SystemDescriptor> {
        todo!()
    }
}
