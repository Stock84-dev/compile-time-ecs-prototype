use core::marker::PhantomData;

use ergnomics::some_loop;
use inception::*;

use crate::{
    loop_index::LoopIndexResource,
    stages::{BacktestInit, Input0},
    types::{Direction, Fee, OrderId, PositionAction, PositionSize, Slippage},
    *,
};

pub mod hlcv_backtest;
pub mod orderflow_backtest;

pub struct BacktestPlugin<I, S> {
    pub inputs: Series<S>,
    pub starting_balance: f32,
    pub slippage: Slippage,
    pub fee: Fee,
    pub inputs_marker: PhantomData<I>,
}

impl<I: Inputs + 'static, S: Nest + 'static> Plugin for BacktestPlugin<I, S> {
    type Deps<L: PluginLoader> = L;

    type Build<B: EcsBuilder> = impl EcsBuilder;

    #[inline(always)]
    fn deps<L: PluginLoader>(&mut self, loader: L) -> Self::Deps<L> {
        loader
    }

    #[inline(always)]
    fn build<B: EcsBuilder>(self, builder: B) -> Self::Build<B> {
        let input = BacktestInput {
            starting_balance: self.starting_balance,
        };
        builder
            .extend_entities(MetricComponent::<Position>::default())
            .extend_entities(MetricComponent::<NOrders>::default())
            .extend_entities(MetricComponent::<EntryPrice>::default())
            .extend_entities(MetricComponent::<ExitPrice>::default())
            .extend_entities(MetricComponent::<PrevBalance>::default())
            .init_resource::<PriceResource>()
            .add_resource(self.slippage)
            .add_resource(self.fee)
            .add_resource(StartingBalance(self.starting_balance))
            .add_system_without_plugin(input::new::<I, S>(self.inputs), Input0::new())
            .add_system_without_plugin(update_prev_balance::new(), Last::new())
            .add_system_without_plugin(init_backtest::new(input), BacktestInit::new())
    }
}

#[system_param]
// Needs to be added with plugin.
pub struct BacktestingParams {
    balance: Metric<Balance>,
    position: Metric<Position>,
    entry_price: Metric<EntryPrice>,
    exit_price: Metric<ExitPrice>,
    fee: Res<Fee>,
    slippage: Res<Slippage>,
    position_updated: EntityEvents<PositionUpdated>,
    position_opened: EntityEvents<PositionOpened>,
    position_closed: EntityEvents<PositionClosed>,
    order_executed: EntityEvents<OrderExecuted>,
    order_canceled: EntityEvents<OrderCanceled>,
    orders: Orders,
}

impl<'w, 's, const N: usize> BacktestingParams<'w, 's, N> {
    #[inline(always)]
    pub fn trade(&mut self, high: f32, low: f32, close: f32) {
        use crate::types::{
            Direction::{Long, Short},
            PositionAction::{Close, Open},
        };
        let price = close;
        let mut state = CoreState {
            position: *self.position,
            entry_price: *self.entry_price,
            balance: *self.balance,
            exit_price: *self.exit_price,
        };
        let slippage = **self.slippage;
        let fee = **self.fee;
        let position_updated = &mut self.position_updated;
        let position_opened = &mut self.position_opened;
        let position_closed = &mut self.position_closed;
        let order_executed = &mut self.order_executed;
        // Function used instead of a closure to force inlining.
        #[inline(always)]
        fn execute<const N: usize>(
            order_id: OrderId,
            position_action: PositionAction,
            direction: Direction,
            price: f32,
            position_size: PositionSize,
            state: &mut CoreState,
            slippage: Slippage,
            fee: Fee,
            position_updated: &mut EntityEvents<PositionUpdated, N>,
            position_opened: &mut EntityEvents<PositionOpened, N>,
            position_closed: &mut EntityEvents<PositionClosed, N>,
            order_executed: &mut EntityEvents<OrderExecuted, N>,
        ) {
            let size = match (position_action, direction) {
                (Open, Long) => state.open_long(price, position_size, slippage, fee),
                (Open, Short) => state.open_short(price, position_size, slippage, fee),
                (Close, Long) => state.close_long(price, position_size, slippage, fee),
                (Close, Short) => state.close_short(price, position_size, slippage, fee),
            };
            // dbg!(state);
            position_updated.send(PositionUpdated {
                position_action,
                direction,
                size,
            });
            match position_action {
                Open => position_opened.send(PositionOpened { direction, size }),
                Close => position_closed.send(PositionClosed { direction, size }),
            }
            order_executed.send(OrderExecuted { id: order_id });
        }
        // let mut execute =
        //     |order_id, position_action, direction, price, position_size, state: &mut CoreState| {
        //         let size = match (position_action, direction) {
        //             (Open, Long) => state.open_long(price, position_size, slippage, fee),
        //             (Open, Short) => state.open_short(price, position_size, slippage, fee),
        //             (Close, Long) => state.close_long(price, position_size, slippage, fee),
        //             (Close, Short) => state.close_short(price, position_size, slippage, fee),
        //         };
        //         // dbg!(state);
        //         position_updated.send(PositionUpdated {
        //             position_action,
        //             direction,
        //             size,
        //         });
        //         match position_action {
        //             Open => position_opened.send(PositionOpened { direction, size }),
        //             Close => position_closed.send(PositionClosed { direction, size }),
        //         }
        //         order_executed.send(OrderExecuted { id: order_id });
        //     };
        let orders = self.orders.get_mut();
        // let c = orders.clone();
        for maybe_order in &mut orders.active_orders.orders {
            let order = some_loop!(maybe_order);
            let id = order.id;
            match &mut order.order {
                OrderPlaced::StopMarket {
                    size,
                    position_action,
                    direction,
                    trigger,
                } => {
                    match position_action {
                        PositionAction::Open => {
                            if state.is_position_opened() {
                                *maybe_order = None;
                                self.order_canceled.send(OrderCanceled { id });
                                continue;
                            }
                        },
                        PositionAction::Close => {
                            if state.is_position_closed() {
                                *maybe_order = None;
                                self.order_canceled.send(OrderCanceled { id });
                                continue;
                            }
                        },
                    }
                    match direction {
                        Short if high > *trigger => {},
                        Long if low < *trigger => {},
                        _ => {
                            continue;
                        },
                    }
                    // println!("stop");
                    execute::<N>(
                        order.id,
                        *position_action,
                        *direction,
                        price,
                        *size,
                        &mut state,
                        slippage,
                        fee,
                        position_updated,
                        position_opened,
                        position_closed,
                        order_executed,
                    );
                },
                _ => {},
            }
        }
        for maybe_order in &mut orders.tmp_orders.orders {
            let mut order = some_loop!(maybe_order.take());
            // dbg!(&order);
            match &mut order.order {
                OrderPlaced::Market {
                    size,
                    position_action,
                    direction,
                } => {
                    execute::<N>(
                        order.id,
                        *position_action,
                        *direction,
                        price,
                        *size,
                        &mut state,
                        slippage,
                        fee,
                        position_updated,
                        position_opened,
                        position_closed,
                        order_executed,
                    );
                    *maybe_order = None;
                },
                _ => {
                    orders.active_orders.push(order);
                },
            }
        }
        // if c != *orders {
        // dbg!(&c, &orders);
        // }
        *self.balance = state.balance;
        *self.position = state.position;
        *self.entry_price = state.entry_price;
        *self.exit_price = state.exit_price;
    }
}

#[derive(Debug)]
struct CoreState {
    position: f32,
    entry_price: f32,
    balance: f32,
    exit_price: f32,
}

impl CoreState {
    #[inline(always)]
    fn is_position_opened(&self) -> bool {
        !self.is_position_closed()
    }

    #[inline(always)]
    fn is_position_closed(&self) -> bool {
        self.position == 0.
    }

    #[inline(always)]
    fn open(&mut self, size: f32, fee: Fee) {
        match fee {
            Fee::RelativeToVolume(x) => {
                self.balance -= size * self.entry_price * x;
            },
        }
    }

    #[inline(always)]
    fn open_long(&mut self, price: f32, size: PositionSize, slippage: Slippage, fee: Fee) -> f32 {
        // println!(
        //     "open long: {:?}, {:?}, {:?} {:?}",
        //     price, size, slippage, fee
        // );
        match slippage {
            Slippage::Relative(x) => {
                self.entry_price = price * (1. + x);
            },
            Slippage::Absolute(x) => {
                self.entry_price = price + x;
            },
        }
        let size = self.get_open_size(size);
        self.position += size;
        self.open(size, fee);
        size
    }

    #[inline(always)]
    fn open_short(&mut self, price: f32, size: PositionSize, slippage: Slippage, fee: Fee) -> f32 {
        // println!(
        //     "open short: {:?}, {:?}, {:?} {:?}",
        //     price, size, slippage, fee
        // );
        match slippage {
            Slippage::Relative(x) => {
                self.entry_price = price * (1. - x);
            },
            Slippage::Absolute(x) => {
                self.entry_price = price - x;
            },
        }
        let size = self.get_open_size(size);
        self.position = -size;
        self.open(size, fee);
        size
    }

    #[inline(always)]
    fn close_long(&mut self, price: f32, size: PositionSize, slippage: Slippage, fee: Fee) -> f32 {
        // println!(
        //     "close long: {:?}, {:?}, {:?} {:?}",
        //     price, size, slippage, fee
        // );
        let exit_price;
        match slippage {
            Slippage::Relative(x) => {
                exit_price = price * (1. - x);
            },
            Slippage::Absolute(x) => {
                exit_price = price - x;
            },
        }
        let size = self.get_close_size(size);
        self.position -= size;
        self.close(exit_price, size, fee);
        size
    }

    #[inline(always)]
    fn close(&mut self, exit_price: f32, size: f32, fee: Fee) -> f32 {
        match fee {
            Fee::RelativeToVolume(x) => {
                self.balance -= exit_price * size * x;
            },
        }
        self.balance += (exit_price - self.entry_price) * size;
        // dbg!(self.balance, exit_price, self.entry_price, size);
        self.exit_price = exit_price;
        self.position = 0.;
        size
    }

    #[inline(always)]
    fn close_short(&mut self, price: f32, size: PositionSize, slippage: Slippage, fee: Fee) -> f32 {
        let exit_price;
        // println!(
        //     "close short: {:?}, {:?}, {:?} {:?}",
        //     price, size, slippage, fee
        // );
        match slippage {
            Slippage::Relative(x) => {
                exit_price = price * (1. + x);
            },
            Slippage::Absolute(x) => {
                exit_price = price + x;
            },
        }
        let size = self.get_close_size(size);
        self.position += size;
        self.close(exit_price, size, fee);
        size
    }

    #[inline(always)]
    fn get_open_size(&self, size: PositionSize) -> f32 {
        match size {
            PositionSize::Relative(x) => self.balance / self.entry_price * x,
            PositionSize::Absolute(x) => x,
        }
    }

    #[inline(always)]
    fn get_close_size(&self, size: PositionSize) -> f32 {
        match size {
            PositionSize::Relative(x) => self.position * x,
            PositionSize::Absolute(x) => x,
        }
    }
}

pub struct BacktestInput {
    starting_balance: f32,
}

#[system]
fn update_prev_balance(balance: Metric<Balance>, mut prev_balance: Metric<PrevBalance>) {
    *prev_balance = *balance;
}

#[system]
fn init_backtest(
    inputs: In<BacktestInput>,
    mut balance: Metric<Balance>,
    mut prev_balance: Metric<PrevBalance>,
) {
    *balance = inputs.starting_balance;
    *prev_balance = inputs.starting_balance;
}

pub struct InputsState<S: Nest>(Series<S>);

impl<S: Nest + 'static> SystemParamState for InputsState<S> {
    #[inline(always)]
    fn init<W: World, SB: SystemParamNameMapper, ParamName: 'static, I: Input>(
        inputs: &mut I,
        _world: &mut W,
    ) -> Self {
        Self(inputs.take::<Series<S>, SB, input::_in>())
    }
}

pub struct InputsParam<'w, 's, I, S, const N: usize> {
    inputs: PhantomData<(I, S)>,
    _marker: PhantomSystemParam<'w, 's, N>,
}

impl<'w, 's,I: Inputs, S: Nest + 'static, const N: usize> SystemParam
    for InputsParam<'w, 's, I, S, N>
{
    // cast lifetimes
    type Item<'world, 'state, Wrld: World> = InputsParam<'world, 'state, I, S, N>;
    type State = InputsState<S>;

    type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> =
        impl EcsBuilder;

    #[inline(always)]
    fn get_param<'world, 'state, Wrld: World, SB: SystemParamNameMapper, ParamName>(
        state: &'state mut Self::State,
        world: &'world mut Wrld,
    ) -> Self::Item<'world, 'state, Wrld> {
        let index = world.resource::<LoopIndexResource>().0;
        I::load(&state.0[index], world);
        InputsParam {
            inputs: PhantomData,
            _marker: Default::default(),
        }
    }

    #[inline(always)]
    fn build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static>(
        builder: B,
    ) -> Self::Build<B, SB, ParamName> {
        builder
    }
}

macro_rules! impl_inputs {
    ($($param:ident),*) => {
        impl<$($param: InputField),*> Inputs for ($($param,)*) {
            // Prepend args with `_` to make `cargo fix` happy.
            #[inline(always)]
            fn load<S: Nest, W: World>(_nest_arg: &S, _world: &mut W) {
                $(_nest_arg.field::<$param>().load(_world.resource_mut::<$param::Resource>());)*
            }
        }
    };
}

all_tuples::all_tuples!(impl_inputs, 0, 16, P);

#[system]
fn input<I: Inputs, S: Nest + 'static>(_inputs: InputsParam<I, S>, _in: PhantomIn<Series<S>>) {
    // Handled in param.
}
