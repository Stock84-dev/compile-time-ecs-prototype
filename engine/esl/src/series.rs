use core::marker::PhantomData;

use crate::schema::SchemaReader;

pub struct Series<T> {
    data: *const u8,
    stride: usize,
    _layout: PhantomData<T>,
}

impl<T> Series<T> {
    pub unsafe fn new(data: *const u8) -> Self {
        Self::with_stride(data, core::mem::size_of::<T>())
    }

    pub unsafe fn with_stride(data: *const u8, stride: usize) -> Self {
        Self {
            data,
            stride,
            _layout: PhantomData,
        }
    }
}

impl<T> SchemaReader<T> for Series<T> {
    unsafe fn read_struct(&mut self, struct_index: usize, struct_size: usize, dest: *mut u8) {
        let src = self.data.add(struct_index * self.stride);
        std::ptr::copy_nonoverlapping(src as *const u8, dest, struct_size);
    }

    unsafe fn read_field(
        &mut self,
        _struct_index: usize,
        _field_offset: usize,
        _field_size: usize,
        _struct_dest: *mut u8,
    ) {
        todo!()
    }
}

impl<T> core::ops::Index<usize> for Series<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            let ptr = self.data.add(index * self.stride);
            &*(ptr as *const T)
        }
    }
}
