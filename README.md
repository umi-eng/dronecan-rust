# DroneCAN

This library provides a pure-Rust implementation of DroneCAN similar to what [libcanard](https://github.com/dronecan/libcanard) is for C/C++.

## Features

- `std` (default) enables the use of slices owned by the library.
- `alloc` enables the use of slices owned by the library.
- `defmt` enables [`defmt`](https://crates.io/crates/defmt) formatting on
  relevant types.

## References

- [Specification](https://dronecan.github.io/Specification/)
