use std::num::NonZeroU32;

pub trait SparsetKey
where
    Self: Clone + Copy + PartialEq + Eq + std::fmt::Debug + 'static
{
    fn index(&self) -> usize;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct SparseSetIndex(pub(crate) Option<NonZeroU32>);

impl SparseSetIndex {
    pub(crate) const fn new(data_index: usize) -> Self {
        unsafe {
            Self(Some(NonZeroU32::new_unchecked(data_index as u32 + 1)))
        }
    }

    pub(crate) fn get(&self) -> Option<usize> {
        self.0.map(|num| (num.get() - 1) as usize)
    }

    pub(crate) const fn null() -> Self {
        Self(None)
    }

    pub(crate) const fn is_valid(&self) -> bool {
        self.0.is_some()
    }
}

impl std::fmt::Debug for SparseSetIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.get() {
            Some(num) => write!(f, "Index({num})"),
            None => write!(f, "Index::null"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SparseIndices(pub(crate) Vec<SparseSetIndex>);

impl Default for SparseIndices {
    fn default() -> Self {
        Self::new()
    }
}

impl SparseIndices {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub unsafe fn get_index_unchecked<K: SparsetKey>(&self, key: K) -> usize {
        (self.0[key.index()].0.unwrap().get() - 1) as usize
    }

    pub fn get_index<K: SparsetKey>(&self, key: K) -> Option<usize> {
        self.0.get(key.index())
            .and_then(SparseSetIndex::get)
    }

    pub fn set_index<K: SparsetKey>(&mut self, key: K, data_index: usize) {
        let index = key.index();
        self.resize_if_needed(index);
        self.0[index] = SparseSetIndex::new(data_index);
    }

    pub fn set_null<K: SparsetKey>(&mut self, key: K) {
        self.0[key.index()] = SparseSetIndex::null()
    }

    pub fn contains<K: SparsetKey>(&self, key: K) -> bool {
        self.0.get(key.index())
            .is_some_and(SparseSetIndex::is_valid)
    }

    fn resize_if_needed(&mut self, key: usize) {
        if key >= self.0.len() {
            self.resize(key);
        }
    }

    pub(crate) fn resize(&mut self, new_len: usize) {
        self.0.resize(new_len + 1, SparseSetIndex::null());
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}
