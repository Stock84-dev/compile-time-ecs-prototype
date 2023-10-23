#![feature(adt_const_params)]
#![feature(const_generics_defaults)]


#[macro_use]
extern crate cuda_std;

#[cfg(target_os = "cuda")]
pub mod cuda;
#[cfg(not(target_os = "cuda"))]
pub mod non_cuda;

use core::mem::size_of_val;
use core::marker::PhantomData;

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy)]
struct Struct {
    a: f32,
    b: f32,
    c: u32,
    d: [f32; 2],
}

impl Writable for Struct {
    fn write(self, writer: &mut impl SchemaWriter<Self>) {
        writer.write_struct(self);
        unsafe {
            writer.write_field(&self.a as *const _ as *const u8, size_of_val(&self.a), 0);
            writer.write_field(&self.b as *const _ as *const u8, size_of_val(&self.b), 4);
            writer.write_field(&self.c as *const _ as *const u8, size_of_val(&self.c), 8);
            writer.write_field(&self.d as *const _ as *const u8, size_of_val(&self.d), 8);
        }
        writer.struct_written();
    }
}

impl Readable for Struct {
    fn read(reader: &mut impl SchemaReader<Self>, struct_index: usize) -> Self {
        unsafe {
            let mut data = core::mem::MaybeUninit::<Self>::uninit().assume_init();
            let struct_ptr = &mut data as *mut _ as *mut u8;
            reader.read_struct(struct_index, core::mem::size_of::<Self>(), struct_ptr);
            reader.read_field(struct_index, 0, core::mem::size_of::<f32>(), struct_ptr);
            reader.read_field(struct_index, 4, core::mem::size_of::<f32>(), struct_ptr);
            reader.read_field(struct_index, 8, core::mem::size_of::<u32>(), struct_ptr);
            reader.read_field(
                struct_index,
                8,
                core::mem::size_of::<[f32; 2]>(),
                struct_ptr,
            );
            data
        }
    }
}

pub trait Writable: Pod {
    fn write(self, writer: &mut impl SchemaWriter<Self>);
}

pub trait SchemaWriter<T> {
    fn write_struct(&mut self, data: T);
    unsafe fn write_field(&mut self, data: *const u8, len: usize, field_offset: usize);
    fn struct_written(&mut self);
}

pub struct AosWriter {
    n_threads: usize,
    dest: *mut u8,
}

impl AosWriter {
    pub fn new(n_threads: usize, dest: *mut u8) -> Self {
        Self { n_threads, dest }
    }
}

impl<T: Pod> SchemaWriter<T> for AosWriter {
    fn write_struct(&mut self, data: T) {
        let output = self.dest as *mut T;
        unsafe {
            *output = data;
            self.dest = self.dest.add(core::mem::size_of::<T>() * self.n_threads);
        }
    }

    unsafe fn write_field(&mut self, _data: *const u8, _len: usize, _field_offset: usize) {}

    fn struct_written(&mut self) {}
}

pub struct SoaWriter {
    array_len: usize,
    struct_offset: usize,
    n_threads: usize,
    dest: *mut u8,
}

impl SoaWriter {
    pub fn new(array_len: usize, n_threads: usize, dest: *mut u8) -> Self {
        Self {
            array_len,
            dest,
            struct_offset: 0,
            n_threads,
        }
    }
}

impl<T: Pod> SchemaWriter<T> for SoaWriter {
    fn write_struct(&mut self, _data: T) {}

    unsafe fn write_field(&mut self, data: *const u8, len: usize, field_offset: usize) {
        let dest = self
            .dest
            .add(self.array_len * field_offset + self.struct_offset * len);
        core::ptr::copy_nonoverlapping(data, dest, len);
    }

    fn struct_written(&mut self) {
        self.struct_offset += self.n_threads;
    }
}

pub struct InlineWriter<T> {
    data: T,
}

impl<T: Pod> SchemaWriter<T> for InlineWriter<T> {
    fn write_struct(&mut self, data: T) {
        self.data = data;
    }

    unsafe fn write_field(&mut self, _data: *const u8, _len: usize, _field_offset: usize) {}

    fn struct_written(&mut self) {}
}

pub trait Writer<T> {
    fn write(&mut self, data: T);
}

impl<W: SchemaWriter<T>, T: Writable> Writer<T> for W {
    fn write(&mut self, data: T) {
        data.write(self);
    }
}

pub trait SchemaReader<T> {
    unsafe fn read_struct(&mut self, struct_index: usize, struct_size: usize, dest: *mut u8);
    unsafe fn read_field(
        &mut self,
        struct_index: usize,
        field_offset: usize,
        field_size: usize,
        struct_dest: *mut u8,
    );
}

pub trait Reader<T> {
    fn read(&mut self, struct_index: usize) -> T;
}

impl<R: SchemaReader<T>, T: Readable> Reader<T> for R {
    fn read(&mut self, struct_offset: usize) -> T {
        <T as Readable>::read(self, struct_offset)
    }
}

pub trait Readable: Pod {
    fn read(reader: &mut impl SchemaReader<Self>, struct_index: usize) -> Self;
}

pub struct AosReader {
    n_threads: usize,
    src: *const u8,
}

impl<T: Pod> SchemaReader<T> for AosReader {
    unsafe fn read_struct(&mut self, struct_index: usize, struct_size: usize, dest: *mut u8) {
        let src = self
            .src
            .add(struct_index * core::mem::size_of::<T>() * self.n_threads);
        core::ptr::copy_nonoverlapping(src, dest, struct_size);
    }

    unsafe fn read_field(
        &mut self,
        _struct_index: usize,
        _field_offset: usize,
        _field_size: usize,
        _struct_dest: *mut u8,
    ) {
    }
}

pub struct SoaReader {
    array_len: usize,
    n_threads: usize,
    src: *const u8,
}

impl<T: Pod> SchemaReader<T> for SoaReader {
    unsafe fn read_struct(&mut self, struct_index: usize, struct_size: usize, dest: *mut u8) {}

    unsafe fn read_field(
        &mut self,
        struct_index: usize,
        field_offset: usize,
        field_size: usize,
        struct_dest: *mut u8,
    ) {
        let src = self
            .src
            .add(self.array_len * field_offset + struct_index * self.n_threads * field_size);
        core::ptr::copy_nonoverlapping(src, struct_dest.add(field_offset), field_size);
    }
}

pub trait Indicator<I, O> {
    fn build(builder: &mut Builder);
    fn start_update_at(&self) -> usize;
    fn update(
        &mut self,
        reader: &mut impl Reader<I>,
        input_index: usize,
        writer: &mut impl Writer<O>,
    );
}

pub struct RsiRollingMem<F> {
    period: F,
    avg_gain: F,
    avg_loss: F,
}

impl<F: Float> RsiRollingMem<F> {
    pub fn new(period: F) -> Self {
        Self {
            period,
            avg_gain: F::zero(),
            avg_loss: F::zero(),
        }
    }

    fn build(period: Param<F, RsiPeriod, 2.0, 1024.0, 1.0>) -> Self {
        Self::new(period.cur())
    }

}

pub trait Float:
    num_traits::Float
    + num_traits::NumAssign
    + num_traits::FromPrimitive
    + num_traits::ToPrimitive
    + num_traits::AsPrimitive<bool>
    + num_traits::AsPrimitive<i8>
    + num_traits::AsPrimitive<i16>
    + num_traits::AsPrimitive<i32>
    + num_traits::AsPrimitive<i64>
    + num_traits::AsPrimitive<i128>
    + num_traits::AsPrimitive<u8>
    + num_traits::AsPrimitive<u16>
    + num_traits::AsPrimitive<u32>
    + num_traits::AsPrimitive<u64>
    + num_traits::AsPrimitive<u128>
{
    fn as_bool(self) -> bool;
    fn as_i8(self) -> i8;
    fn as_i16(self) -> i16;
    fn as_i32(self) -> i32;
    fn as_i64(self) -> i64;
    fn as_i128(self) -> i128;
    fn as_isize(self) -> isize;
    fn as_u8(self) -> u8;
    fn as_u16(self) -> u16;
    fn as_u32(self) -> u32;
    fn as_u64(self) -> u64;
    fn as_u128(self) -> u128;
    fn as_usize(self) -> usize;

    fn of_bool(b: bool) -> Self;
    fn of_i8(b: i8) -> Self;
    fn of_i16(b: i16) -> Self;
    fn of_i32(b: i32) -> Self;
    fn of_i64(b: i64) -> Self;
    fn of_i128(b: i128) -> Self;
    fn of_isize(b: isize) -> Self;
    fn of_u8(b: u8) -> Self;
    fn of_u16(b: u16) -> Self;
    fn of_u32(b: u32) -> Self;
    fn of_u64(b: u64) -> Self;
    fn of_u128(b: u128) -> Self;
    fn of_usize(b: usize) -> Self;
}

impl<F: Float + Readable> Indicator<F, F> for RsiRollingMem<F> {
    fn build(builder: &mut Builder) {
        builder.add(Self::build);
    }

    fn start_update_at(&self) -> usize {
        self.period.as_usize() + 1usize
    }

    fn update(
        &mut self,
        reader: &mut impl Reader<F>,
        input_index: usize,
        writer: &mut impl Writer<F>,
    ) {
        let price = reader.read(input_index);
        let prev_price = reader.read(input_index - 1);
        let diff = price - prev_price;
        let last_price = reader.read(input_index - self.period.as_usize());
        let last_prev_price = reader.read(input_index - self.period.as_usize() - 1);
        let last_diff = last_price - last_prev_price;
        // Using rolling average because it is faster, but it is prone to prcision errors
        // First remove from average to minimize floating point precision errors
        self.avg_gain -= F::of_bool(last_diff > F::zero()) * last_diff / self.period;
        self.avg_loss += F::of_bool(last_diff < F::zero()) * last_diff / self.period;

        self.avg_gain += F::of_bool(diff > F::zero()) * diff / self.period;
        self.avg_loss -= F::of_bool(diff < F::zero()) * diff / self.period;

        let mut rs = self.avg_gain / self.avg_loss;
        rs = if rs.is_nan() { F::one() } else { rs };
        let rsi = F::of_i32(100) - (F::of_i32(100) / (F::one() + rs));
        writer.write(rsi);
    }
}
#[cfg(esl_data_test)]
#[data_test]
// generates a random walk, tests if the output is the same if calculation would begin towards the
// end of the walk rather than starting it from the beginning
mod data_tests {
    fn rsi_rolling_mem<F: Float>(rsi: RsiRollingMemItem<F>) -> bool {
        rsi >= F::of_i32(0) && rsi <= F::of_i32(100)
    }
}
// TODO: running multiple models of different functions
// TODO: precision mapping
// TODO: generic executor (CUDA executor, CPU executor, CUDA-cache executor, CPU-cache executor)

struct Gstring<const S: &'static str>();
struct RsiRollingMemItem<F>(F);

struct Or<F>(F);
struct And<F>(F);

#[model]
/// Model documentation.
fn demo<F>(model: impl Model<F>) -> impl Model<F> {
    model.open_long(buy_signal)
        .open_short(sell_signal)
        .close_long(Or((fixed_long_stop_loss, tp_long)))
        .close_short(Or((fixed_short_stop_loss, tp_short)))
}

pub trait Run<F> {
    fn run(model: &mut Model<F>);
}

struct FixedLongStopLoss;

// const params get parsed out and replaced with the actual number
// I don't like the idea of this, wehen backtesting
#[stop_long]
/// Signal documentation.
fn fixed_long_stop_loss<const STOP_LOSS: Param<f64, StopLoss, 0.001, 0.1, 0.001>, F>(
    entry_price: EntryPrice<F>,
    price: Price<F>,
) -> (bool, F) {
    let stop_price = entry_price.0 * F::of_f64(1.0 - STOP_LOSS);
    (price.0 < stop_price, stop_price)
}

#[stop_short]
fn fixed_short_stop_loss<F>(
    entry_price: EntryPrice<F>,
    price: Price<F>,
    stop_loss: Param<F, StopLoss, 0.001, 0.1, 0.001>,
) -> bool {
    price.0 > entry_price.0 * (1.0 + stop_loss)
}

struct Hline;
struct Lline;
struct StopLoss;

#[tp_long]
fn tp_long<F>(rsi: Prev<RsiRollingMemItem<F>>, lline: Param<F, Lline, 0.0, 100.0, 1.0>) -> bool {
    rsi.cur() < lline && rsi.prev() > lline
}

#[tp_short]
fn tp_short<F>(rsi: Prev<Rsi RollingMemItem<F>>, hline: Param<F, Hline, 0.0, 100.0, 1.0>) -> bool {
    rsi.cur() > hline && rsi.prev() < hline
}                                          

#[open_long]  
fn buy_signal<F>(rsi: Prev<RsiRollingMemItem<F>>, lline: Param<F, Lline>) -> bool {
    rsi.cur() < lline && rsi.prev() > lline
}
    
#[open_short]
fn sell_signal<F>(rsi: Prev<RsiRollingMemItem<F>>, hline: Param<F, Hline>) -> bool {
    rsi.cur() > hline && rsi.prev() < hline
}                            

pub struct Prev<T>(T);

impl<T: StrategyParam> StrategyParam for Prev<T> {
    fn deps();
    fn build();
    fn get();
    fn tick();
}

fn backtest(app: App) {


}


struct RsiRollingMemInput<F>(PhantomData<F>);
struct RsiRollingMemSeries<F>(PhantomData<F>);

struct RsiRollingMemItem<F> {
    value: F,
}

struct RsiRollingMemItemState<F, R, W> {
    indicator: RsiRollingMem<F, F>,
    reader: R,
    writer: W,
}

impl<F> StrategyParam for RsiRollingMemItem<F> {
    fn deps(req: &mut Require, provide: &mut Provide) {
        let input_id = req.series(RsiRollingMemInput(PhandomData::<F>::default()));
        let output_id = provide.series(RsiRollingMemSeries(PhandomData::<F>::default()));
        provide.indicator::<RsiRollingMem<F>>(input_id, output_id, |world: &World| -> &mut F {
                &mut world.get_resource_mut::<Self>().value;
        });
    }

    fn build(world: &mut World) -> Self {
        let s = Self {f: 0.0};
        // first check if resource is there in any upper levels
        world.get_reader::<RsiRollingMemInput<F>>();
        world.get_writer::<RsiRollingMemSeries<F>>();
        world.insert_resource()
        world.insert(ConstParam())

    }
}
  
struct RsiItem<F>(F);
  
    // declare constParam to pick indicator calculation
impl StrategyParam for RsiItem {
    fn deps(deps: &mut Deps) {
        deps.insert(ConstParam())
    }

    fn build(world: &mut World) -> Self {
        match world.const_params.get::<RsiCalculator>().unwrap().0 {
            IndicatorOptimization::RollingMem => 

        }
        world.insert(ConstParam())

    }
}
// Compile paramaters: 
// pick calculator for indicators
// direction filter: long only, short only, long and short
// precison: f16, f32, f64
// cache specific indicator or generate it on the fly
//
// Data paramaters (requires loading data in different format):
// Backtest on high or low or close or volume

struct RsiCalculator(IndicatorOptimization);
enum IndicatorOptimization {
    RollingMem,
    Rolling,
    Convolution,
}

struct ConstParamAlias();

fn strategy(
    rsi: Prev<RsiRollingMemItem<F>>, 
    hline: Param<F, Hline, 0.0, 100.0, 1.0>,
    lline: Param<F, Lline, 0.0, 100.0, 1.0>,
    stop_loss: Param<F, StopLoss, 0.001, 0.1, 0.001>,
)  {  

}
  
fn b() {}

#[derive(PartialEq, Eq)]
#[repr(transparent)]
struct Wrap<T>(T);

struct Container<const F: Wrap<fn()>>();

type My = Container<{Wrap(b)}>;

struct MyType(f32);
impl PartialEq for MyType {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for MyType {}
struct AB<const A: MyType>();


// Cannot use floats as const parameters
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct ConstFloat(i32);
impl core::ops::Add<f32> for ConstFloat {
    type Output = f32;
    fn add(self, other: f32) -> Self::Output {
        unsafe { core::mem::transmute::<i32, f32>(self.0) + other }
    }
}
struct Param<F, Name, const START: ConstFloat = {ConstFloat::new(0.0)}, const END: OrderedFloat<f32> = {OrderedFloat(0.0)}, const STRIDE: OrderedFloat<f32> = {OrderedFloat(0.0)}> {
    f: PhantomData<F>,
    name: PhantomData<Name>,
}

fn a(c: Container<{ Wrap(b) }>) {}
fn d(c: My) {}

struct C<F> {
    f: F,
}

impl<F: Fn(f32) -> f32> C<F> {
    pub fn run(&self, a: f32) -> f32 {
        (self.f)(a)
    }
}

pub fn j<F: Fn(f32) -> f32>(c: C<F>) -> f32 {
    c.run(f)
}


pub fn k(eval: impl Fn(f32) -> f32, i: f32) -> f32 {
    eval(i) * 1.2345
}

struct Wrapper<F, I> {
    f: F,
    i: I,
}
pub trait Run {
    fn run(&mut self, value: f32) -> f32;
}
impl<F: FnMut(f32) -> f32, I: Run> Run for Wrapper<F, I> {
    fn run(&mut self, value: f32) -> f32 {
        self.i.run((self.f)(value))
    }
}
pub struct Empty;
impl Run for Empty {
    fn run(&mut self, value: f32) -> f32 {
        value
    }
}
fn inc(value: f32) -> f32 {
    value + 1.0
}

pub fn wrapper() -> f32 {
    let mut value = 0;
    let mut w = Wrapper {
        f: inc,
        i: Wrapper {
            f: inc2,
            i: Empty,
        },
    };
    w.run(value)
}

pub fn co() -> f32 {
    let vec: Vec<fn(f32) -> f32> = vec![inc, inc];
    let mut value = 0.0;
    for i in vec {
        value = i(value);
    }
    value
}

fn unroll<const N_ITERS: usize, F: FnMut(usize)>(f: F) {

    }

pub trait ResourceContainer {
    fn get_resource<'a, T: 'static>(&'a self) -> Option<&'a T>;
    }

pub struct EmptyResource;

pub struct ResourceStruct<T, R> {
    value: T,
        inner: R,
    }

impl<T: 'static, R: ResourceContainer> ResourceContainer for ResourceStruct<T, R> {
    fn get_resource<'a, S: 'static>(&'a self) -> Option<&'a S> {
            if core::any::TypeId::of::<T>() == core::any::TypeId::of::<S>() {
                unsafe {
                    return Some(&*(&self.value as *const _ as *const S));
                }
            }
            self.inner.get_resource::<S>()
    }
}

impl ResourceContainer for EmptyResource {
    fn get_resource<S>(&self) -> Option<&S> {
        None
    }
}

    pub struct ResourceGroup {

    }

pub fn resources() -> i32 {
        let mut resources = ResourceStruct {
            value: "hello",
            inner: ResourceStruct {
                value: 42,
                inner: EmptyResource,
            },
        };
        *resources.get_resource::<i32>().unwrap()
    }

struct B(i32);

fn a<const P: B>() -> i32 {
    P.0
}
