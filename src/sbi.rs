//! SBI Implementation.
//! Ref: [https://github.com/rustsbi/rustsbi-qemu](https://github.com/rustsbi/rustsbi-qemu)

use rustsbi::RustSBI;

#[derive(RustSBI)]
struct Sbi {}
