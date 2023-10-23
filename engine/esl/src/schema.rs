use bytemuck::Pod;

impl Readable for f32 {
    fn read(reader: &mut impl SchemaReader<Self>, struct_index: usize) -> Self {
        unsafe {
            #[allow(clippy::uninit_assumed_init, invalid_value)]
            let mut data = core::mem::MaybeUninit::<Self>::uninit().assume_init();
            let struct_ptr = &mut data as *mut _ as *mut u8;
            reader.read_struct(struct_index, core::mem::size_of::<Self>(), struct_ptr);
            data
        }
    }
}

// #[repr(C)]
// #[derive(Pod, Zeroable, Clone, Copy)]
// struct Struct {
//     a: f32,
//     b: f32,
//     c: u32,
//     d: [f32; 2],
// }
//
// impl Writable for Struct {
//     fn write(self, writer: &mut impl SchemaWriter<Self>) {
//         writer.write_struct(self);
//         unsafe {
//             writer.write_field(&self.a as *const _ as *const u8, size_of_val(&self.a), 0);
//             writer.write_field(&self.b as *const _ as *const u8, size_of_val(&self.b), 4);
//             writer.write_field(&self.c as *const _ as *const u8, size_of_val(&self.c), 8);
//             writer.write_field(&self.d as *const _ as *const u8, size_of_val(&self.d), 8);
//         }
//         writer.struct_written();
//     }
// }
//
// impl Readable for Struct {
//     fn read(reader: &mut impl SchemaReader<Self>, struct_index: usize) -> Self {
//         unsafe {
//             let mut data = core::mem::MaybeUninit::<Self>::uninit().assume_init();
//             let struct_ptr = &mut data as *mut _ as *mut u8;
//             reader.read_struct(struct_index, core::mem::size_of::<Self>(), struct_ptr);
//             reader.read_field(struct_index, 0, core::mem::size_of::<f32>(), struct_ptr);
//             reader.read_field(struct_index, 4, core::mem::size_of::<f32>(), struct_ptr);
//             reader.read_field(struct_index, 8, core::mem::size_of::<u32>(), struct_ptr);
//             reader.read_field(
//                 struct_index,
//                 8,
//                 core::mem::size_of::<[f32; 2]>(),
//                 struct_ptr,
//             );
//             data
//         }
//     }
// }

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
        let src = self.src.add(struct_index * struct_size * self.n_threads);
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
    unsafe fn read_struct(&mut self, _struct_index: usize, _struct_size: usize, _dest: *mut u8) {}

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

// #[cfg(esl_data_test)]
// #[data_test]
// // generates a random walk, tests if the output is the same if calculation would begin towards
// the // end of the walk rather than starting it from the beginning
// mod data_tests {
//     fn rsi_rolling_mem<F: Float>(rsi: RsiRollingMemItem<F>) -> bool {
//         rsi >= F::of_i32(0) && rsi <= F::of_i32(100)
//     }
// }
