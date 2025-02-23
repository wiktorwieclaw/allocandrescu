# allocandrescu
[![Crates.io](https://img.shields.io/crates/v/allocandrescu.svg)](https://crates.io/crates/allocandrescu)
[![Released API docs](https://docs.rs/allocandrescu/badge.svg)](https://docs.rs/allocandrescu)
[![Continuous integration](https://github.com/wiktorwieclaw/allocandrescu/actions/workflows/ci.yaml/badge.svg?branch=main)](https://github.com/wiktorwieclaw/allocandrescu/actions/workflows/ci.yaml)

A collection of various allocators and allocator combinators inspired by [Andrei Alexandrescu](https://en.wikipedia.org/wiki/Andrei_Alexandrescu)'s CppCon 2015 talk [std::allocator Is to Allocation what std::vector Is to Vexation](https://www.youtube.com/watch?v=LIb3L4vKZ7U) and the [Zig programming language](https://ziglang.org/).

`allocandrescu` allows you to safely compose allocators using combinators such as
`cond` and `fallback`.
It also provides a bunch of simple allocators like `Stack`.

This crate depends on [`allocator-api2`](https://crates.io/crates/allocator-api2), a polyfill for the unstable [`allocator_api`](https://doc.rust-lang.org/unstable-book/library-features/allocator-api.html) feature.
