# Compile-time ECS prototype
This project demonstrates how to create an ECS using zero-cost abstractions. Almost every ECS operation is optimized away. 

Features:
- No memory allocations
- Passing parameters to systems is zero-cost
- `#![no_std]` compatible
- Single-threaded
- Custom schedule with multiple loops
- Plugins, systems, events and other smaller features
- Components and resources must be inserted at compile-time

Pros:
- Provides highly modular code without any cost

Cons:
- Not suitable in an environment where the number of entities is unknown at compile-time.
- Long compile times. Incremental build of `trackers` example takes 1m 20s and 10 GiB of RAM.

## Core concept
Everything revolves around nested types. When an archetype is empty it has a type of `StackedNest`. By adding a component it grows into `Nested<MyComponent, StackedNest>`, then `Nested<MyComponent, Nested<MyComponent, StackedNest>>`...
See `inception/inception/src/nest_module.rs`

A method is called on every nested struct when passing parameters to systems. This method checks if the provided type is the same as type contained in a struct.
See `inception/inception/src/entities.rs`
Rust compiler will optimize those checks because components are added at compile-time.

## Architecture
Main ECS crate is `inception`. Usage is in `esl`.

## Installation
Nightly is required because the old code uses it and `inception` uses `type_alias_impl_trait` to simplify the return types of an associated type in traits simpler. 

## Documentation
There are some docs for `esl` and `inception`.
```bash
cargo doc -p esl --open
```

## Examples
```bash
# to get a list of examples
cargo build --example
# to build a specific example
cargo build --example=custom_indicator
```
