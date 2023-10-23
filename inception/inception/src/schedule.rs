use crate::{stage::StageBuilder, *};

macro_rules! def_stage_items {
    ($f:ident, $ret:ident) => {
        type $ret<System: SystemBuilder<'static, 'static> + 'static>: ScheduleBuilderTrait
            + StageBuilder;
        #[must_use]
        fn $f<System>(self, system: System) -> Self::$ret<System>
        where
            System: SystemBuilder<'static, 'static> + 'static;
    };
}
// Ends with `Trait` to avoid name clashing with schedule macro.
pub trait ScheduleBuilderTrait {
    all_tuples::repeat!(
        def_stage_items,
        0,
        32,
        add_system_to_stage,
        AddSystemToStage
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{plugin::CorePlugin, resources::Break, stages::*};
    #[system]
    fn system_a(mut value: Res<i32>) {
        assert_eq!(**value, 0);
        **value += 1;
    }
    #[system]
    fn system_ba(mut value: Res<i32>, index: Res<usize>) {
        if **index == 0 {
            assert_eq!(**value, 1);
        } else {
            assert_eq!(**value, 6);
        }
        **value += 1;
    }

    #[system]
    fn system_bb(mut value: Res<i32>, index: Res<usize>) {
        if **index == 0 {
            assert_eq!(**value, 2);
        } else {
            assert_eq!(**value, 7);
        }
        **value *= 2;
    }

    #[system]
    fn system_bc(mut value: Res<i32>, mut index: Res<usize>, mut break_loop: Res<Break>) {
        if **index == 0 {
            assert_eq!(**value, 4);
        } else {
            assert_eq!(**value, 14);
            **break_loop = Break(true);
        }
        **index += 1;
        **value += 2;
    }

    #[system]
    fn system_ca(mut value: Res<i32>) {
        assert_eq!(**value, 16);
        **value += 2;
    }

    #[system]
    fn system_cb(mut value: Res<i32>) {
        assert_eq!(**value, 18);
        **value *= 2;
    }

    #[system]
    fn system_cc(mut value: Res<i32>) {
        assert_eq!(**value, 36);
        **value += 1;
    }

    pub type A = Stage0;
    pub type BA = Stage1;
    pub type BB = Stage2;
    pub type BC = Stage3;
    pub type CA = Stage4;
    pub type CB = Stage5;
    pub type CC = Stage6;

    schedule! {
        struct Schedule,
        Stage0 as A,
        loop {
            Stage1 as BA,
            Stage2 as BB,
            Stage3 as BC,
        },
        Stage4 as CA,
        Stage5 as CB,
        Stage6 as CC,
    }
    #[test]
    pub fn schedule() {
        let schedule = Schedule::builder();
        let ecs = EcsBuilderStruct::new::<_, 0>(ConfigBuilder::new(), schedule)
            .add_plugin(CorePlugin)
            .insert_resource(0i32)
            .insert_resource(0usize)
            .add_system_to_stage(system_a::new(), A::new())
            .add_system_to_stage(system_ba::new(), BA::new())
            .add_system_to_stage(system_bb::new(), BB::new())
            .add_system_to_stage(system_bc::new(), BC::new())
            .add_system_to_stage(system_ca::new(), CA::new())
            .add_system_to_stage(system_cb::new(), CB::new())
            .add_system_to_stage(system_cc::new(), CC::new());
        let mut ecs = ecs.build();
        ecs.run();
        let value = ecs.get_resource::<i32>().unwrap();
        assert_eq!(*value, 37);
        let value = ecs.get_resource::<usize>().unwrap();
        assert_eq!(*value, 2);
        let value = ecs.get_resource::<Break>().unwrap();
        assert_eq!(value.0, false);
    }
}
