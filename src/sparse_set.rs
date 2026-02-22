use std::ptr::NonNull;

use crate::buffer::RawBuffer;
use crate::error::Error;
use crate::indices::DataIndices;
use crate::iterator::{Iter, IterMut};

/// A container which holds the data contiguously, consistent indexing with integer (usize), and no element shifting on middle removal.
/// 
/// # Example
/// 
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
///
/// // new push will reuse previously removed index if available
/// let new = s.push("f");
/// assert_eq!(new, a);
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
            data_indexes: DataIndices::with_capacity(capacity),
            len: 0,
        }
    }

    /// Move and transform an existing Vec\<V\> into a SparseSet.
    /// 
    /// # Example
    /// 
    /// ```ignore
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
        let data_indexes = DataIndices::from_slice(&keys);

        Self {
            data: RawBuffer::from_raw(vec.as_ptr(), len),
            keys: RawBuffer::from_raw(keys.as_ptr(), len),
            data_indexes,
            len,
        }
    }


    /// Create a new SparseSet from a slice.
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// use sparse_set::SparseSet;
    ///
    /// let arr = [0, 1, 2, 3, 4];
    ///
    /// let s = SparseSet::from_arr(arr);
    ///
    /// assert_eq!(s.values(), arr);
    /// ```
    pub fn from_arr<const N: usize>(arr: [V; N]) -> Self {
        let keys = (0..N).into_iter().collect::<Box<[_]>>();
        let data_indexes = DataIndices::from_slice(&keys);

        Self {
            data: RawBuffer::from_raw(arr.as_ptr(), N),
            keys: RawBuffer::from_raw(keys.as_ptr(), N),
            data_indexes,
            len: N,
        }
    }

    /// Create a new SparseSet from a slice.
    /// 
    /// # Example
    /// 
    /// ```ignore
    /// use sparse_set::SparseSet;
    ///
    /// let vec = vec![0, 1, 2, 3, 4];
    /// 
    /// let s = SparseSet::from_slice(&vec);
    ///
    /// assert_eq!(s.values(), [0, 1, 2, 3, 4]);
    /// ```
    pub fn from_slice(slice: &[V]) -> Self {
        let len = slice.len();
        let keys = (0..len).into_iter().collect::<Box<[_]>>();
        let data_indexes = DataIndices::from_slice(&keys);

        Self {
            data: RawBuffer::from_raw(slice.as_ptr(), len),
            keys: RawBuffer::from_raw(keys.as_ptr(), len),
            data_indexes,
            len,
        }
    }

    /// This won't realloc if len is equal capacity, an error is returned instead.
    /// This is intended to be used with [`with_capacity`](Self::with_capacity).
    /// 
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
        if self.data.check(self.len).is_err() {
            self.data.grow();
            self.keys.grow();
        }

        unsafe { self.push_inner(value) }
    }

    unsafe fn push_inner(&mut self, value: V) -> usize {
        let index = if self.data_indexes.len() > self.len {
            unsafe {
                let empty = *self.keys.get_raw(self.len);
                self.data_indexes.set(empty, self.len);
                self.data.push(value, self.len);

                empty
            }
        } else {
            self.data_indexes.push(self.len);

            unsafe {
                self.data.push(value, self.len);
                self.keys.push(self.len, self.len);
            }

            self.len
        };

        self.len += 1;
        index
    }

    /// Remove an element at a specified index. The validity of the indexes to access other elements are preserved.
    /// The contiguousness of the data is also preserved, but internally the order of the data is modified,
    /// because the vacant spot of the removed element is filled by the last element.
    /// 
    /// # Example
    /// 
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
    /// // the order of the elements is modified to accomodate contiguousness of the data
    /// // and avoid the significant cost of shifting the elements after the removed element
    /// let removed = s.remove(4);
    /// assert_eq!(s.values(), [0, 1, 2, 3, 7, 5, 6]);
    ///
    /// // although the order of the elements is modified, the index to access the element is still valid
    /// let seventh = s.get(7);
    /// assert_eq!(seventh, Some(&7));
    /// ```
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

    /// Remove the last element
    pub fn pop(&mut self) -> Option<V> {
        (!self.is_empty()).then(|| {
            let last = unsafe { self.data.pop(self.len).read() };
            self.len -= 1;
            last
        })
    }

    /// Get an Option of raw pointer of the contained value at the specified index.
    /// 
    /// # Safety
    /// 
    /// Handle the produced raw pointer with care!
    pub unsafe fn get_raw(&self, index: usize) -> Option<NonNull<V>> {
        self.data_indexes
            .get(index)
            .and_then(|data_index| unsafe {
                let ptr = self.data.get_raw(data_index);
                NonNull::new(ptr)
            })
    }

    /// Get an immutable reference of the contained value at the specified index while skipping all the necessary check.
    /// 
    /// # Safety
    /// 
    /// THe caller must ensure the validity of the index.
    pub unsafe fn get_unchecked(&self, index: usize) -> &V {
        unsafe {
            let data_index = self.data_indexes.get_unchecked(index);
            &*self.data.get_raw(data_index)
        }
    }

    /// Get an immutable reference of the contained value at the specified index.
    pub fn get(&self, index: usize) -> Option<&V> {
        self.data_indexes
            .get(index)
            .map(|data_index| unsafe {
                &*self.data
                    .get_raw(data_index)
            })
    }

    /// Get a mutable reference of the contained value at the specified index while skipping all the necessary check.
    /// 
    /// # Safety
    /// 
    /// THe caller must ensure the validity of the index.
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut V {
        unsafe {
            let data_index = self.data_indexes.get_unchecked(index);
            &mut *self.data.get_raw(data_index)
        }
    }

    /// Get a mutable reference of the contained value at the specified index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut V> {
        self.data_indexes
            .get(index)
            .map(|data_index| unsafe {
                &mut *self.data
                    .get_raw(data_index)
            })
    }

    /// Get an immutable reference to the first element
    pub fn first(&self) -> Option<&V> {
        if self.len == 0 {
            return None;
        }

        unsafe {
            Some(self.data.ptr.as_ref())
        }
    }

    /// Get a mutable reference to the first element
    pub fn first_mut(&mut self) -> Option<&mut V> {
        if self.len == 0 {
            return None;
        }

        unsafe {
            Some(self.data.ptr.as_mut())
        }
    }

    /// Get an immutable reference to the last element
    pub fn last(&self) -> Option<&V> {
        if self.len == 0 {
            return None;
        }

        unsafe {
            Some(self.data.ptr.add(self.len - 1).as_ref())
        }
    }

    /// Get a mutable reference to the last element
    pub fn last_mut(&mut self) -> Option<&mut V> {
        if self.len == 0 {
            return None;
        }

        unsafe {
            Some(self.data.ptr.add(self.len - 1).as_mut())
        }
    }

    /// Get an immutable slice of the contained values
    pub fn values<'a>(&'a self) -> &'a [V] {
        unsafe {
            &*std::ptr::slice_from_raw_parts(
                self.data.ptr.as_ptr().cast_const(),
                self.len
            )
        }
    }

    /// Get an immutable slice of the valid indexes
    pub fn indexes<'a>(&'a self) -> &'a [usize] {
        unsafe {
            &*std::ptr::slice_from_raw_parts(
                self.keys.ptr.as_ptr().cast_const(),
                self.len
            )
        }
    }

    /// Clear the values and invalidate the indexes
    pub fn clear(&mut self) {
        if self.len > 0 {
            self.data_indexes.clear();
            self.data.clear(self.len);
            self.keys.clear(self.len);
            self.len = 0;
        }
    }

    /// Return the current capacity of this SparseSet
    pub const fn capacity(&self) -> usize {
        self.keys.capacity
    }

    /// Return the amount of elements currently stored in this SparseSet
    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return an immutable iterator over the contained values
    ///
    /// # Example
    ///
    /// ```
    /// use sparse_set::{SparseSet, sparse};
    ///
    /// let s = sparse![1, 2, 3];
    /// let mut iter = s.iter();
    /// 
    /// assert_eq!(iter.next(), Some(&1));
    /// assert_eq!(iter.next(), Some(&2));
    /// assert_eq!(iter.next(), Some(&3));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn iter(&self) -> Iter<'_, V> {
        Iter::new(self.data.ptr.as_ptr(), self.len())
    }

    /// Return a mutable iterator over the contained values
    ///
    /// # Example
    ///
    /// ```
    /// use sparse_set::{SparseSet, sparse};
    ///
    /// let mut s = sparse![1, 2, 3];
    /// let mut iter = s.iter_mut();
    /// 
    /// assert_eq!(iter.next(), Some(&mut 1));
    /// assert_eq!(iter.next(), Some(&mut 2));
    /// assert_eq!(iter.next(), Some(&mut 3));
    /// assert_eq!(iter.next(), None);
    /// ```
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
            .entries(self.indexes().iter().zip(self.values()))
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
            && self.indexes() == other.indexes()
            && self.data_indexes == other.data_indexes
    }
}

impl<V: Eq> Eq for SparseSet<V> {}

impl<V: Clone> Clone for SparseSet<V> {
    fn clone(&self) -> Self {
        Self {
            data: RawBuffer::from_raw(self.data.ptr.as_ptr().cast_const(), self.len),
            keys: RawBuffer::from_raw(self.keys.ptr.as_ptr().cast_const(), self.len),
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
            data: RawBuffer::from_raw(items.as_ptr(), len),
            keys: RawBuffer::from_raw(keys.as_ptr(), len),
            data_indexes: DataIndices::from_slice(&keys),
            len,
        }
    }
}
