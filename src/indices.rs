use std::num::NonZeroU32;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Index(Option<NonZeroU32>);

impl Index {
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
}

impl std::fmt::Debug for Index {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.get() {
            Some(num) => write!(f, "{num}"),
            None => write!(f, "null"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DataIndices(Vec<Index>);

impl DataIndices {
    pub(crate) const fn new() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub(crate) fn from_slice(slice: &[usize]) -> Self {
        Self(slice.iter().map(|i| Index::new(*i)).collect::<Vec<_>>())
    }

    pub(crate) fn push(&mut self, index: usize) {
        self.0.push(Index::new(index))
    }

    pub(crate) fn set(&mut self, index: usize, data_index: usize) {
        unsafe {
            *self.0.get_unchecked_mut(index) = Index::new(data_index);
        }
    }

    pub(crate) fn set_null(&mut self, index: usize) {
        unsafe {
            *self.0.get_unchecked_mut(index) = Index::null()
        }
    }

    pub(crate) unsafe fn get_unchecked(&self, index: usize) -> usize {
        unsafe {
            self.0.get_unchecked(index).get().unwrap()
        }
    }

    pub(crate) fn get(&self, index: usize) -> Option<usize> {
        self.0.get(index).and_then(Index::get)
    }

    #[inline(always)]
    pub(crate) const fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }
}
