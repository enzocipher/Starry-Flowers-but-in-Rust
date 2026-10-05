# Local PSP allocator patch

Source: `psp` 0.3.14 from crates.io, upstream https://github.com/overdrivenpotato/rust-psp. The upstream MIT license is included in `LICENSE`.

Changes:

- Replace the per-allocation PSP partition allocator in `src/alloc_impl.rs` with `linked_list_allocator::LockedHeap`.
- Export the unsafe, one-time `init_heap(size)` initializer, called before allocating game code.
- Add the `linked_list_allocator` dependency.

The original allocator exhausted PSP kernel object slots while deserializing the story. The game reserves one 16 MiB arena instead. Its audio workers use preallocated buffers and paths, so they do not contend for the allocator across PSP threads.

Other SDK source files and the supplied unwind libraries are retained from the crate. SDK warnings from newer nightly compilers are not treated as application warnings.
