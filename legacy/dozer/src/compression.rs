// use bevy::{
// prelude::*,
// reflect::{
// FromReflect,
// serde::{ReflectDeserializer, ReflectSerializer},
// DynamicStruct, TypeRegistry,
// },
// };
// use std::io::Write;
// use std::io::Read;
// use serde::de::DeserializeSeed;
//
// This example illustrates how "reflection" works in Bevy. Reflection provide a way to dynamically
// interact with Rust types, such as accessing fields by their string name. Reflection is a core
// part of Bevy and enables a number of interesting scenarios (like scenes).
// fn main() {
// App::new()
// .add_plugins(DefaultPlugins)
// .register_type::<Foo>()
// .register_type::<Ivan>()
// .add_startup_system(setup)
// .run();
// }
//
// Deriving `Reflect` implements the relevant reflection traits. In this case, it implements the
// `Reflect` trait and the `Struct` trait `derive(Reflect)` assumes that all fields also implement
// Reflect.
// #[derive(Reflect, FromReflect)]
// pub struct Foo {
// a: usize,
// nested: Ivan,
// #[reflect(ignore)]
// _ignored: NonReflectedValue,
// }
//
// This `Ivan` type is used in the `nested` field on the `Test` type. We must derive `Reflect` here
// too (or ignore it)
// #[derive(Reflect, FromReflect)]
// pub struct Ivan {
// b: usize,
// }
//
// pub struct NonReflectedValue {
// _a: usize,
// }
//
// fn setup(type_registry: Res<TypeRegistry>) {
// let mut value = Foo {
// a: 1,
// _ignored: NonReflectedValue { _a: 10 },
// nested: Ivan { b: 8 },
// };
//
// You can set field values like this. The type must match exactly or this will fail.
// value.get_field_mut("a").unwrap() = 2usize;
// assert_eq!(value.a, 2);
// assert_eq!(*value.get_field::<usize>("a").unwrap(), 2);
//
// You can also get the &dyn Reflect value of a field like this
// let field = value.field("a").unwrap();
//
// you can downcast Reflect values like this:
// assert_eq!(*field.downcast_ref::<usize>().unwrap(), 2);
//
// DynamicStruct also implements the `Struct` and `Reflect` traits.
// let mut patch = DynamicStruct::default();
// patch.insert("a", 4usize);
//
// You can "apply" Reflect implementations on top of other Reflect implementations.
// This will only set fields with the same name, and it will fail if the types don't match.
// You can use this to "patch" your types with new values.
// value.apply(&patch);
// assert_eq!(value.a, 4);
//
// let type_registry = type_registry.read();
// By default, all derived `Reflect` types can be Serialized using serde. No need to derive
// Serialize!
// let serializer = ReflectSerializer::new(&value, &type_registry);
// let ron_string =
// ron::ser::to_string_pretty(&serializer, ron::ser::PrettyConfig::default()).unwrap();
// let mut file = std::fs::File::create("a").unwrap();
// file.write_all(ron_string.as_bytes()).unwrap();
// let mut file = std::fs::File::open("a").unwrap();
// let mut ron_string = String::new();
// file.read_to_string(&mut ron_string).unwrap();
// info!("{}\n", ron_string);
//
// Dynamic properties can be deserialized
// let reflect_deserializer = ReflectDeserializer::new(&type_registry);
// let mut deserializer = ron::de::Deserializer::from_str(&ron_string).unwrap();
// let reflect_value = reflect_deserializer.deserialize(&mut deserializer).unwrap();
// let a: &dyn Reflect = &*reflect_value;
//
// use bevy::reflect::Reflect;
// use bevy::reflect::FromReflect;
//
// Deserializing returns a Box<dyn Reflect> value. Generally, deserializing a value will return
// the "dynamic" variant of a type. For example, deserializing a struct will return the
// DynamicStruct type. "Value types" will be deserialized as themselves.
// let _deserialized_struct = Reflect::downcast::<DynamicStruct>(*a).unwrap();
// let foo = Foo::from_reflect(a).unwrap();
//
// let foo = _deserialized_struct.downcast::<Foo>().unwrap();
//
// Reflect has its own `partial_eq` implementation, named `reflect_partial_eq`. This behaves
// like normal `partial_eq`, but it treats "dynamic" and "non-dynamic" types the same. The
// `Foo` struct and deserialized `DynamicStruct` are considered equal for this reason:
// assert!(reflect_value.reflect_partial_eq(&value).unwrap());
//
// By "patching" `Foo` with the deserialized DynamicStruct, we can "Deserialize" Foo.
// This means we can serialize and deserialize with a single `Reflect` derive!
// value.apply(&*reflect_value);
// panic!();
// }
use std::io::Write;
use std::mem::swap;
use std::sync::Arc;

use bevy::ecs::schedule::{IntoSystemDescriptor, SystemDescriptor};
use mouse::sync::Mutex;
use zion::prelude::*;
use zion::{read_all, Zion};

use crate::common::BytesMessage;

pub struct CompressionPlugin;
impl ZionPlug for CompressionPlugin {
    fn deps<'a, 'b>(&mut self, loader: &'a mut PluginLoader<'b>) -> &'a mut PluginLoader<'b> {
        loader
    }

    fn load<'a>(&mut self, zion: &'a mut Zion) -> &'a mut Zion {
        zion
    }
}

#[derive(Serialize, Deserialize)]
pub enum SizeUnit {
    B = 1,
    KiB = (1 << 10),
    MiB = (1 << 20),
    GiB = (1 << 30),
    TiB = (1 << 40),
    PiB = (1 << 50),
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[non_exhaustive]
#[repr(u8)]
pub enum CompressionAlgorithm {
    Lzma,
}

const_assert_eq!(CompressionAlgorithm::Lzma as u8, 0);

pub trait Compressor: Send + Sync + 'static {
    fn write_all(&mut self, data: &[u8]) -> std::io::Result<()>;
    fn finish(self) -> std::io::Result<Vec<u8>>;
    fn get_mut(&mut self) -> &mut Vec<u8>;
}

impl Compressor for xz2::write::XzEncoder<Vec<u8>> {
    fn write_all(&mut self, data: &[u8]) -> std::io::Result<()> {
        std::io::Write::write_all(self, data)
    }

    fn finish(self) -> std::io::Result<Vec<u8>> {
        self.finish()
    }

    fn get_mut(&mut self) -> &mut Vec<u8> {
        self.get_mut()
    }
}

#[derive(Serialize, Deserialize)]
pub struct CompressAccumulateArgs {
    algorithm: CompressionAlgorithm,
    level: i64,
    threshold: usize,
    unit: SizeUnit,
}

pub struct CompressAccumulate {
    args: CompressAccumulateArgs,
    reader: TopicReader<BytesMessage>,
    writer: TopicWriter<BytesMessage>,
    compressor: Box<dyn Compressor>,
    message_id: i64,
}

impl Default for CompressAccumulate {
    fn default() -> Self {
        // systems require that local arguments implement default
        panic!("default cannot be implemented")
    }
}

fn build_compressor(algorithm: CompressionAlgorithm, level: i64) -> Box<dyn Compressor> {
    match algorithm {
        CompressionAlgorithm::Lzma => {
            Box::new(xz2::write::XzEncoder::new(Vec::new(), level as u32))
        }
    }
}

impl CompressAccumulate {
    fn system(local: Local<Arc<Mutex<CompressAccumulate>>>) {
        let local = local.lock();
        for e in read_all!(local.reader) {
            local.compressor.write_all(&e.data).unwrap();
            if local.compressor.get_mut().len() > local.args.threshold * local.args.unit as usize {
                let mut compressor = build_compressor(local.args.algorithm, local.args.level);
                swap(&mut compressor, &mut local.compressor);
                let data = compressor.finish().unwrap();
                local.writer.write(BytesMessage {
                    data: Arc::new(data),
                    mesage_id: local.message_id,
                });
                local.message_id += 1;
            }
        }
    }
}

#[async_trait]
impl Pipe for CompressAccumulate {
    fn layout() -> PipeLayout
    where
        Self: Sized,
    {
        PipeLayout {
            static_estimations: None,
            kind: PipeKind::Bevy {
                stage: CoreStage::Update,
            },
        }
    }

    fn new<'a>(builder: &mut ParamBuilder<'a>) -> AnyResult<Arc<dyn Pipe>>
    where
        Self: Sized,
    {
        let args: CompressAccumulateArgs = builder.deserialize_args()?;
        Ok(Arc::new(Self {
            reader: builder.get_reader()?,
            writer: builder.get_writer()?,
            compressor: build_compressor(args.algorithm, args.level),
            args,
            message_id: 0,
        }))
    }

    async fn spawn(self: Arc<Self>) -> AnyResult<()> {
        Ok(())
    }

    fn system(&self) -> Option<SystemDescriptor> {
        Some(
            Self::system
                .system()
                //                .config(|x| x.0 = Some(self.clone()))
                .into_descriptor(),
        )
    }
}
