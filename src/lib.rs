//! SparseSet is a container which aims to get the best of both array or Vec and linked-list.
//! 
//! # Array
//! Data locality of a simple array is superior compared to linked-list. Indexing over an array is also superior compared to a linked-list.
//! Unfortunately, removing an element somewhere in the middle an array incur a costly performance penalty.
//! Because the other elements after the removed one need to be shifted to maintain the density.
//! Using a swap-remove with the last element will obviously invalidate the index because the order of the element if modified.
//!
//! # Linked-list
//! Meanwhile the efficiency of a linked-list on middle element removal is superior compared to an array.
//! 
//! # SparseSet
//! This container will hold the data contiguously, can be consistently indexed using an integer, and no element shifting on middle removal.
//!
//! # Examples
//! ```ignore
//! use sparse_set::SparseSet;
//!
//! let mut s = SparseSet::<&'static str>::with_capacity(5);
//!
//! let a = s.push("a");
//! let b = s.push("b");
//! let c = s.push("c");
//! let d = s.push("d");
//! let e = s.push("e");
//!
//! assert_eq!([a, b, c, d, e], [0, 1, 2, 3, 4]);
//!
//! let removed_a = s.remove(a);
//! assert_eq!(removed_a, Some("a"));
//! assert_eq!(s.get(a), None);
//! assert_eq!(s.values(), ["e", "b", "c", "d"]);
//!
//! // index 4 still points to "e"
//! let get_4 = s.get(4);
//! assert_eq!(get_4, Some(&"e"));
//!
//! let indexed_4 = s[4];
//! assert_eq!(indexed_4, &"e");
//! ```
//!
//! ### Macro
//! A helper macro is also provided for convenient.
//!
//! ```ignore
//! use sparse_set::{SparseSet, sparse};
//!
//! let a = sparse![0, 1, 2, 3];
//! let b = SparseSet::from_arr([0, 1, 2, 3]);
//!
//! assert_eq!(a, b);
//! ```
//!
//! ### Iterator
//! A built-in iterator is also available.
//!
//! ```ignore
//! use sparse_set::{SparseSet, sparse};
//!
//! let a = sparse![0, 1, 2, 3];
//!
//! // iterating over a SparseSet
//! let b = a.iter().collect::<Vec<_>>();
//! let vec = vec![0, 1, 2, 3];
//!
//! assert_eq!(b, vec);
//!
//! // collecting from an iterator
//! let c = vec.iter().collect::<SparseSet<_>>();
//! assert_eq!(c, a);
//!
//! ```

mod indices;
mod sparse_set;
mod buffer;
mod iterator;

pub use sparse_set::SparseSet;

pub mod error {
    #[derive(Debug)]
    pub enum Error {
        ExceedCurrentCapacity,
        Uninitialized,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{self:?}")
        }
    }

    impl std::error::Error for Error {}
}

#[macro_export]
/// Helper macro to quickly create a new SparseSet. Similar to how vec![] works.
macro_rules! sparse {
    [] => [
        SparseSet::new()
    ];
    [$elem:expr; $n:expr] => [
        SparseSet::from_arr([$elem; $n])
    ];
    [$($x:expr),+ $(,)?] => [
        SparseSet::from_slice(&[$($x),+])
    ];
}

#[cfg(test)]
mod util_test {
    use super::*;

    #[test]
    fn macro_test() {
        let a = sparse![69, 69, 69, 69, 69];
        let b = sparse![69; 5];

        assert_eq!(a, b);
    }

    #[test]
    fn clone_test() {
        let a = sparse![69; 5];
        let b = a.clone();

        assert_eq!(a, b);
    }

    #[test]
    fn from_iter() {
        let a = [0; 8];
        let b = a.iter().copied().collect::<SparseSet<_>>();

        assert_eq!(b.values(), &a);
    }
}
