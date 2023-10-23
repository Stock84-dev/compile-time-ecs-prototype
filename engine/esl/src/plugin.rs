use inception::{resources::Break, *};

use crate::{
    loop_index::LoopIndexResource,
    stages::{IncLoopIndex, IncPreLoopIndex},
    LoopEndBoundExcluded, LoopIndex,
};

pub struct CorePlugin {
    /// Set this to the number of candles provided.
    pub loop_end_bound_excluded: usize,
}

impl Plugin for CorePlugin {
    type Build<B: EcsBuilder> = impl EcsBuilder;
    type Deps<L: PluginLoader> = impl PluginLoader;

    #[inline(always)]
    fn deps<L: PluginLoader>(&mut self, loader: L) -> Self::Deps<L> {
        loader.load_once(inception::CorePlugin)
    }

    #[inline(always)]
    fn build<B: EcsBuilder>(self, builder: B) -> Self::Build<B> {
        builder
            .add_resource(LoopIndexResource(0))
            .add_resource(LoopEndBoundExcluded(self.loop_end_bound_excluded))
            .add_system_without_plugin(inc_loop_index::new(), IncPreLoopIndex::new())
            .add_system_without_plugin(inc_loop_index::new(), IncLoopIndex::new())
    }
}

#[system]
fn inc_loop_index(mut index: LoopIndex, mut break_: Res<Break>, end: Res<LoopEndBoundExcluded>) {
    *index += 1;
    if *index >= end.0 {
        **break_ = Break(true);
    }
}
