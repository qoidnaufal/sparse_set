use std::ptr::NonNull;

use crate::buffer::RawBuffer;
use crate::error::Error;
use crate::indices::DataIndices;
use crate::iterator::{Iter, IterMut};

/// A container which holds the data contiguously, consistent indexing with integer (usize), and no element shifting on middle removal.
/// # Example
/// ```
/// use sparse_set::SparseSet;
///
/// let mut s = SparseSet::<&'static str>::with_capacity(5);
/// let a = s.push("a");
/// let b = s.push("b");
/// let c = s.push("c");
/// let d = s.push("d");
/// let e = s.push("e");
/// 
/// assert_eq!([a, b, c, d, e], [0, 1, 2, 3, 4]);
/// 
/// let removed_a = s.remove(a);
/// assert_eq!(removed_a, Some("a"));
/// assert_eq!(s.get(a), None);
/// assert_eq!(s.values(), ["e", "b", "c", "d"]);
///
/// // index 4 still points to "e"
/// let get_4 = s.get(4);
/// assert_eq!(get_4, Some(&"e"));
///
/// let indexed_4 = s[4];
/// assert_eq!(indexed_4, "e");
/// ```
/// 
/// # Helper macro
/// A helper macro is also provided for convenient
///
/// # Example
/// 
/// ```
/// use sparse_set::{SparseSet, sparse};
/// 
/// let a = sparse![0, 1, 2, 3, 4];
/// let b = SparseSet::from_vec(vec![0, 1, 2, 3, 4]);
///
/// assert_eq!(a, b);
/// ```
pub struct SparseSet<V> {
    data: RawBuffer<V>,
    keys: RawBuffer<usize>,
    data_indexes: DataIndices,
    len: usize,
}

impl<V> SparseSet<V> {
    /// Create a new uninitialized SparseSet.
    pub const fn new() -> Self {
        Self {
            data: RawBuffer::new(),
            keys: RawBuffer::new(),
            data_indexes: DataIndices::new(),
            len: 0,
        }
    }

    /// Create and initialize a new SparseSet with the specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: RawBuffer::with_capacity(capacity),
            keys: RawBuffer::with_capacity(capacity),
            data_indexes: DataIndices::default(),
            len: 0,
        }
    }

    /// Move and transform an existing Vec\<V\> into a SparseSet.
    /// # Example
    /// ```
    /// use sparse_set::SparseSet;
    ///
    /// let vec = vec![0, 1, 2, 3, 4];
    /// 
    /// let s = SparseSet::from_vec(vec);
    ///
    /// assert_eq!(s.values(), [0, 1, 2, 3, 4]);
    /// ```
    pub fn from_vec(vec: Vec<V>) -> Self {
        let len = vec.len();
        let keys = (0..len).into_iter().collect::<Box<[_]>>();
        let data_indexes = DataIndices::from_arr(&keys);

        Self {
            data: RawBuffer::from_ptr(vec.as_ptr(), len),
            keys: RawBuffer::from_ptr(keys.as_ptr(), len),
            data_indexes,
            len,
        }
    }

    pub unsafe fn get_raw(&self, index: usize) -> Option<NonNull<V>> {
        self.data_indexes
            .get(index)
            .and_then(|data_index| unsafe {
                let ptr = self.data.get_raw(data_index);
                NonNull::new(ptr)
            })
    }

    pub unsafe fn get_unchecked(&self, index: usize) -> &V {
        unsafe {
            let data_index = self.data_indexes.get_unchecked(index);
            &*self.data.get_raw(data_index)
        }
    }

    /// Get a reference of stored value at an index.
    pub fn get(&self, index: usize) -> Option<&V> {
        self.data_indexes
            .get(index)
            .map(|data_index| unsafe {
                &*self.data
                    .get_raw(data_index)
            })
    }

    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut V {
        unsafe {
            let data_index = self.data_indexes.get_unchecked(index);
            &mut *self.data.get_raw(data_index)
        }
    }

    /// Get a mutable reference of stored value at an index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut V> {
        self.data_indexes
            .get(index)
            .map(|data_index| unsafe {
                &mut *self.data
                    .get_raw(data_index)
            })
    }

    /// This won't realloc if len is equal capacity. This is intended to be used with [`with_capacity`](Self::with_capacity).
    /// # Example
    /// ```
    /// use sparse_set::SparseSet;
    /// 
    /// let mut s = SparseSet::with_capacity(5);
    /// 
    /// for i in 0..5 {
    ///     let res = s.push_within_capacity(i);
    ///     assert!(res.is_ok());
    /// }
    ///
    /// assert_eq!(s.capacity(), 5);
    /// assert_eq!(s.values(), [0, 1, 2, 3, 4]);
    /// ```
    ///
    /// # Time complexity
    ///
    /// Takes amortized *O*(1) time.
    pub fn push_within_capacity(&mut self, value: V) -> Result<usize, Error> {
        self.data.check(self.len)
            .map(|_| unsafe { self.push_inner(value) })
    }

    /// Appends an element to the back of a this container.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `isize::MAX` _bytes_.
    ///
    /// # Examples
    ///
    /// ```
    /// use sparse_set::SparseSet;
    /// 
    /// let mut s = SparseSet::new();
    /// 
    /// for i in 0..5 {
    ///     s.push(i);
    /// }
    ///
    /// assert_eq!(s.capacity(), 8);
    /// assert_eq!(s.len(), 5);
    /// assert_eq!(s.values(), [0, 1, 2, 3, 4]);
    /// ```
    ///
    /// # Time complexity
    ///
    /// Takes amortized *O*(1) time. If the sparseset's length would exceed its
    /// capacity after the push, *O*(*capacity*) time is taken to copy the
    /// vector's elements to a larger allocation.
    pub fn push(&mut self, value: V) -> usize {
        self.grow_if_needed(self.len);

        unsafe { self.push_inner(value) }
    }

    unsafe fn push_inner(&mut self, value: V) -> usize {
        if self.data_indexes.len() > self.len {
            unsafe {
                let empty = *self.keys.get_raw(self.len);
                self.data_indexes.set(empty, self.len);
                self.data.push(value, self.len);

                self.len += 1;
                return empty;
            }
        }

        let data_index = self.len;
        self.data_indexes.set(data_index, data_index);

        unsafe {
            self.data.push(value, data_index);
            self.keys.push(data_index, data_index);
        }

        self.len += 1;
        data_index
    }

    /// Remove an element at a specified index. The contiguousness of the data is preserved.
    /// The validity of the index is also preserved.
    /// # Example
    /// ```
    /// use sparse_set::SparseSet;
    ///
    /// let mut s = SparseSet::with_capacity(8);
    ///
    /// for i in 0..8 {
    ///     s.push_within_capacity(i).unwrap();
    /// }
    ///
    /// assert_eq!(s.values(), [0, 1, 2, 3, 4, 5, 6, 7]);
    ///
    /// let seventh = s.get(7);
    /// assert_eq!(seventh, Some(&7));
    ///
    /// let removed = s.remove(4);
    /// assert_eq!(s.values(), [0, 1, 2, 3, 7, 5, 6]);
    ///
    /// let seventh = s.get(7);
    /// assert_eq!(seventh, Some(&7));
    /// ```
    ///
    /// # Time complexity
    ///
    /// Takes amortized *O*(1) time.
    pub fn remove(&mut self, index: usize) -> Option<V> {
        if self.is_empty() { return None; }

        let len = self.len;
        let last_key = unsafe { *self.keys.get_raw(len - 1) };

        self.data_indexes.get(index).map(|data_index| {
            self.data_indexes.set(last_key, data_index);
            self.data_indexes.set_null(index);
            self.keys.swap(last_key, data_index);
            self.len -= 1;
            
            unsafe {
                self.data
                    .swap_remove(index, len - 1)
                    .read()
            }
        })
    }

    pub fn pop(&mut self) -> Option<V> {
        (!self.is_empty()).then(|| {
            let last = unsafe { self.data.pop(self.len).read() };
            self.len -= 1;
            last
        })
    }

    pub fn values<'a>(&'a self) -> &'a [V] {
        unsafe {
            &*std::ptr::slice_from_raw_parts(
                self.data.ptr.as_ptr().cast_const(),
                self.len
            )
        }
    }

    fn keys<'a>(&'a self) -> &'a [usize] {
        unsafe {
            &*std::ptr::slice_from_raw_parts(
                self.keys.ptr.as_ptr().cast_const(),
                self.len
            )
        }
    }

    fn grow_if_needed(&mut self, len: usize) {
        if self.data.check(len).is_err() {
            self.data.grow();
            self.keys.grow();
        }
    }

    pub fn clear(&mut self) {
        if self.len > 0 {
            self.data_indexes.clear();
            self.data.clear(self.len);
            self.keys.clear(self.len);
            self.len = 0;
        }
    }

    pub const fn capacity(&self) -> usize {
        self.keys.capacity
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> Iter<'_, V> {
        Iter::new(self.data.ptr.as_ptr(), self.len())
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, V> {
        IterMut::new(self.data.ptr.as_ptr(), self.len())
    }
}

impl<V> Default for SparseSet<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> Drop for SparseSet<V> {
    fn drop(&mut self) {
        self.clear();
        self.data.dealloc();
        self.keys.dealloc();
    }
}

impl<V> std::fmt::Debug for SparseSet<V>
where
    V: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.keys().iter().zip(self.values()))
            .finish()
    }
}

impl<V> std::ops::Index<usize> for SparseSet<V> {
    type Output = V;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            self.get_unchecked(index)
        }
    }
}

impl<V> std::ops::IndexMut<usize> for SparseSet<V> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {
            self.get_unchecked_mut(index)
        }
    }
}

impl<V: PartialEq> PartialEq for SparseSet<V> {
    fn eq(&self, other: &Self) -> bool {
        self.values() == other.values()
            && self.keys() == other.keys()
            && self.data_indexes == other.data_indexes
    }
}

impl<V: Eq> Eq for SparseSet<V> {}

impl<V: Clone> Clone for SparseSet<V> {
    fn clone(&self) -> Self {
        Self {
            data: RawBuffer::from_ptr(self.data.ptr.as_ptr().cast_const(), self.len),
            keys: RawBuffer::from_ptr(self.keys.ptr.as_ptr().cast_const(), self.len),
            data_indexes: self.data_indexes.clone(),
            len: self.len,
        }
    }
}

impl<V> FromIterator<V> for SparseSet<V> {
    fn from_iter<T: IntoIterator<Item = V>>(iter: T) -> Self {
        let items = iter.into_iter().collect::<Box<[_]>>();
        let len = items.len();
        let keys = (0..len).into_iter().collect::<Box<[_]>>();

        Self {
            data: RawBuffer::from_ptr(items.as_ptr(), len),
            keys: RawBuffer::from_ptr(keys.as_ptr(), len),
            data_indexes: DataIndices::from_arr(&keys),
            len,
        }
    }
}
