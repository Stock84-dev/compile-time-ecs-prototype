use derive_more::{Deref, DerefMut};

#[derive(Deref, DerefMut, Clone, Default)]
/// If a tracker has recorded a sample.
pub struct SampleRecorded(pub bool);
#[derive(Deref, DerefMut, Clone, Default)]
/// The current index of a sample.
pub struct SampleId(pub usize);
