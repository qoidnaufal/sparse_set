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
        SparseSet::from_vec(vec![$elem; $n])
    ];
    [$($x:expr),+ $(,)?] => [
        SparseSet::from_vec(vec![$($x),+])
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
    fn iter() {
        let a = [0; 8];
        let b = a.iter().copied().collect::<SparseSet<_>>();

        assert_eq!(b.values(), &a);
    }
}
