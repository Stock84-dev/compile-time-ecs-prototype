use inception::*;

use crate::{
    events::OrderPlaced,
    metrics::Position,
    types::{OrderId, PositionAction},
    value::Value,
    IntoOrder, Metric, NOrders, Price,
};

#[derive(Debug, Clone, PartialEq)]
pub struct OrdersContainer<const N: usize> {
    pub orders: [Option<ActiveOrder>; N],
}

impl<const N: usize> Default for OrdersContainer<N> {
    #[inline(always)]
    fn default() -> Self {
        const ORDER: Option<ActiveOrder> = None;
        OrdersContainer { orders: [ORDER; N] }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveOrder {
    pub id: OrderId,
    pub order: OrderPlaced,
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct OrdersComponent<const N: usize> {
    pub tmp_orders: OrdersContainer<N>,
    pub active_orders: OrdersContainer<N>,
}

pub struct Orders<'w, 's, const N: usize> {
    position: Metric<'w, 's, Position, N>,
    current_price: f32,
    n_orders: Metric<'w, 's, NOrders, N>,
    orders: &'w mut OrdersComponent<N>,
    _marker: PhantomSystemParam<'w, 's, N>,
}

impl<'w, 's, const N: usize> SystemParam for Orders<'w, 's, N> {
    type Item<'world, 'state, Wrld: World> = Orders<'world, 'state, N>;
    type State = ();

    type Build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static> =
        impl EcsBuilder;

    unimpl_get_param!();

    #[inline(always)]
    fn get_param_for_entity<'world, 'state, Wrld, SB, ParamName, E>(
        entity: &'world mut E,
        state: &'state mut Self::State,
        world: &'world mut Wrld,
    ) -> Option<Self::Item<'world, 'state, Wrld>>
    where
        Wrld: World,
        SB: SystemParamNameMapper,
        E: EntityFetch,
        ParamName: 'static,
    {
        let current_price =
            Price::<N>::get_param_for_entity::<Wrld, SB, ParamName, E>(entity, state, world)?.get();
        let entity = entity as *mut E;
        let world = world as *mut Wrld;
        let state = state as *mut Self::State;
        unsafe {
            let position = Metric::<Position, N>::get_param_for_entity::<Wrld, SB, ParamName, E>(
                &mut *entity,
                &mut *state,
                &mut *world,
            )?;
            let n_orders = Metric::<NOrders, N>::get_param_for_entity::<Wrld, SB, ParamName, E>(
                &mut *entity,
                &mut *state,
                &mut *world,
            )?;
            Some(Orders {
                orders: (&mut *entity).component_mut::<OrdersComponent<N>>(),
                _marker: PhantomSystemParam::default(),
                position,
                current_price,
                n_orders,
            })
        }
    }

    fn build<B: EcsBuilder, SB: SystemParamNameMapper + 'static, ParamName: 'static>(
        builder: B,
    ) -> Self::Build<B, SB, ParamName> {
        builder.extend_entities(OrdersComponent::<N>::default())
    }
}

impl<'w, 's, const N: usize> Orders<'w, 's, N> {
    #[inline(always)]
    pub fn on(&mut self, condition: bool, order: impl IntoOrder) -> Option<OrderId> {
        if condition { self.send(order) } else { None }
    }

    #[inline(always)]
    pub fn send(&mut self, order: impl IntoOrder) -> Option<OrderId> {
        let order = order.into_order(self.current_price);
        match &order {
            OrderPlaced::Market {
                position_action,
                direction,
                ..
            } => match position_action {
                PositionAction::Open if self.position.metric().is_opened() => return None,
                PositionAction::Close => {
                    let discard = match self.position.metric().direction() {
                        Some(d) => d != *direction,
                        None => true,
                    };
                    if discard {
                        return None;
                    }
                },
                _ => {},
            },
            OrderPlaced::StopMarket {
                position_action,
                direction: _,
                size: _,
                trigger: _,
            } => match position_action {
                PositionAction::Close if self.position.metric().is_opened() => return None,
                _ => {},
            },
        }
        let id = OrderId(*self.n_orders);
        *self.n_orders += 1;
        self.orders.tmp_orders.push(ActiveOrder { id, order });
        Some(id)
    }

    #[inline(always)]
    pub fn cancel(&mut self, id: OrderId) {
        for maybe_order in &mut self.orders.active_orders.orders {
            if let Some(order) = maybe_order {
                if order.id == id {
                    *maybe_order = None;
                    return;
                }
            }
        }
        panic!("Order with id: {:?} doesn't exist", id);
    }

    #[inline(always)]
    pub fn get(&self) -> &OrdersComponent<N> {
        &self.orders
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut OrdersComponent<N> {
        &mut self.orders
    }
}

impl<const N: usize> OrdersContainer<N> {
    #[inline(always)]
    pub fn push(&mut self, order: ActiveOrder) {
        let id = self.orders.iter().position(|o| o.is_none());
        if let Some(id) = id {
            self.orders[id] = Some(order);
        } else {
            // println!("{:#?}", self.orders);
            panic!("Too many orders per iteration");
        }
    }
}
