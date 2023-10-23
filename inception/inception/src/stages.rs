use crate::{stage::StageLabel, *};

pub type Last = Stage31;

macro_rules! def_stage {
    ($stage:ident, $f:ident, $ret:ident) => {
        pub struct $stage;

        impl $stage {
            #[inline(always)]
            pub fn new() -> Self {
                $stage
            }
        }

        impl StageLabel for $stage {
            type AddSystem<System: SystemBuilder<'static, 'static> + 'static, Builder: EcsBuilder> =
                Builder::$ret<System>;

            #[inline(always)]
            fn add_system<System, Builder>(
                system: System,
                builder: Builder,
            ) -> Self::AddSystem<System, Builder>
            where
                Builder: EcsBuilder,
                System: SystemBuilder<'static, 'static> + 'static,
            {
                builder.$f(system)
            }
        }
    };
}

all_tuples::repeat!(
    def_stage,
    0,
    32,
    Stage,
    add_system_to_stage,
    AddSystemToStage
);
