use std::alloc;
use std::marker::PhantomData;
use std::ptr::NonNull;

use blob_array::raw_buffer::RawBuffer;
use blob_array::error::Error;
use blob_array::iterator::{Iter, IterMut};
use blob_array::util::needs_drop;

use crate::indices::{SparseIndices, SparsetKey};

pub struct SparseSet<K, V> {
    pub(crate) data: RawBuffer,
    pub(crate) keys: Vec<K>,
    pub(crate) indexes: SparseIndices,
    marker: PhantomData<V>
}

impl<K, V> Drop for SparseSet<K, V> {
    fn drop(&mut self) {
        self.clear();
        self.data.dealloc(Self::LAYOUT);
    }
}

impl<K, V> std::fmt::Debug for SparseSet<K, V>
where
    K: std::fmt::Debug,
    V: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.keys().iter().zip(self.iter()))
            .finish()
    }
}

impl<K, V> SparseSet<K, V> {
    const LAYOUT: alloc::Layout = alloc::Layout::new::<V>();

    pub fn keys(&self) -> &[K] {
        &self.keys
    }

    pub fn values<'a>(&'a self) -> &'a [V] {
        unsafe {
            &*std::ptr::slice_from_raw_parts(
                self.data.cast::<V>().cast_const(),
                self.len()
            )
        }
    }

    /// # Safety
    /// This is unsafe, because removing element(s) from this slice should be done from [`SparseSet::swap_remove`]
    /// Only use this method to quickly iterates over the mutable slice of values
    pub unsafe fn values_mut<'a>(&'a mut self) -> &'a mut [V] {
        unsafe {
            &mut *std::ptr::slice_from_raw_parts_mut(
                self.data.cast::<V>(),
                self.len()
            )
        }
    }

    pub fn clear(&mut self) {
        if self.keys.len() > 0 {
            self.indexes.clear();
            self.data.clear(self.keys.len());
            self.keys.clear();
        }
    }

    pub const fn capacity(&self) -> usize {
        self.data.capacity()
    }

    pub const fn len(&self) -> usize {
        self.keys.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> Iter<'_, V> {
        Iter::new(self.data.cast::<V>(), self.len())
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, V> {
        IterMut::new(self.data.cast::<V>(), self.len())
    }
}

impl<K: SparsetKey, V: 'static> SparseSet<K, V> {
    pub const fn new() -> Self {
        Self {
            data: RawBuffer::new(&Self::LAYOUT, needs_drop::<V>()),
            keys: Vec::new(),
            indexes: SparseIndices::new(),
            marker: PhantomData,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: RawBuffer::with_capacity(&Self::LAYOUT, needs_drop::<V>(), capacity),
            keys: Vec::with_capacity(capacity),
            indexes: SparseIndices::default(),
            marker: PhantomData,
        }
    }

    pub unsafe fn get_raw(&self, key: K) -> Option<NonNull<V>> {
        self.indexes
            .get_index(key)
            .and_then(|index| unsafe {
                let ptr = self.data.get_raw(index * Self::LAYOUT.size()).cast();
                NonNull::new(ptr)
            })
    }

    pub unsafe fn get_unchecked(&self, key: K) -> &V {
        unsafe {
            let index = self.indexes.get_index_unchecked(key);
            &*self.data.get_raw(index * Self::LAYOUT.size()).cast()
        }
    }

    pub fn get(&self, key: K) -> Option<&V> {
        self.indexes
            .get_index(key)
            .map(|index| unsafe {
                &*self.data
                    .get_raw(index * Self::LAYOUT.size())
                    .cast()
            })
    }

    pub unsafe fn get_unchecked_mut(&mut self, key: K) -> &mut V {
        unsafe {
            let index = self.indexes.get_index_unchecked(key);
            &mut *self.data.get_raw(index * Self::LAYOUT.size()).cast()
        }
    }

    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.indexes
            .get_index(key)
            .map(|index| unsafe {
                &mut *self.data
                    .get_raw(index * Self::LAYOUT.size())
                    .cast()
            })
    }

    pub fn get_data_index(&self, key: K) -> Option<usize> {
        self.indexes.get_index(key)
    }

    fn grow_if_needed(&mut self, len: usize) {
        if let Err(_) = self.data.check(len) {
            self.data.grow(&Self::LAYOUT, self.data.capacity() + 4);
        }
    }

    /// Safety: you have to ensure len < capacity, and the entity does not existed yet within this sparse_set
    pub unsafe fn insert_unchecked(&mut self, key: K, value: V, len: usize) {
        self.indexes.set_index(key, len);
        unsafe { self.data.push(value, len); }
        self.keys.push(key);
    }

    pub fn insert_within_capacity(
        &mut self,
        key: K,
        value: V,
    ) -> Result<(), Error> {
        if let Some(exist) = self.get_mut(key) {
            *exist = value;
            return Ok(());
        }

        let len = self.len();
        self.data.check(len)?;

        Ok(unsafe { self.insert_unchecked(key, value, len) })
    }

    pub fn insert(&mut self, key: K, value: V) {
        if let Some(exist) = self.get_mut(key) {
            *exist = value;
            return;
        }

        let len = self.len();
        self.grow_if_needed(len);

        unsafe { self.insert_unchecked(key, value, len) }
    }

    pub fn swap_remove(&mut self, key: K) -> Option<V> {
        if self.is_empty() { return None; }

        let last_key = *self.keys.last().unwrap();
        let len = self.len();

        self.indexes.get_index(key).map(|index| {
            self.indexes.set_index(last_key, index);
            self.indexes.set_null(key);
            self.keys.swap_remove(index);
            
            unsafe {
                self.data
                    .swap_remove_or_pop(index, len - 1, Self::LAYOUT.size())
                    .cast::<V>()
                    .read()
            }
        })
    }

    pub fn contains_key(&self, key: K) -> bool {
        self.indexes.contains(key)
    }
}
