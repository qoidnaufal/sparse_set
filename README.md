# SparseSet
A container which holds the data contiguously, consistent indexing with integer (usize), and no element shifting on middle removal.

### Example
```rust
use sparse_set::SparseSet;

let mut s = SparseSet::<&'static str>::with_capacity(5);

let a = s.push("a");
let b = s.push("b");
let c = s.push("c");
let d = s.push("d");
let e = s.push("e");

assert_eq!([a, b, c, d, e], [0, 1, 2, 3, 4]);

let removed_a = s.remove(a);
assert_eq!(removed_a, Some("a"));
assert_eq!(s.get(a), None);
assert_eq!(s.values(), ["e", "b", "c", "d"]);

// index 4 still points to "e"
let get_4 = s.get(4);
assert_eq!(get_4, Some(&"e"));

let indexed_4 = s[4];
assert_eq!(indexed_4, &"e");
```

### Macro
A helper macro is also provided for convenient.
```rust
use sparse_set::{SparseSet, sparse};

let a = sparse![0, 1, 2, 3];
let b = SparseSet::from_vec(vec![0, 1, 2, 3]);

assert_eq!(a, b);
```

### Iterator
A built-in iterator is also available.
```rust
use sparse_set::{SparseSet, sparse};

let a = sparse![0, 1, 2, 3];
let b = a.iter().collect::<Vec<_>>();
let vec = vec![0, 1, 2, 3];

assert_eq!(b, vec);
  
```

### Benchmark
No benchmark has been done
