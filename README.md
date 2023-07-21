# cancel-safe-futures

[![cancel-safe-futures on crates.io](https://img.shields.io/crates/v/cancel-safe-futures)](https://crates.io/crates/cancel-safe-futures)
[![Documentation (latest release)](https://img.shields.io/badge/docs-latest%20version-brightgreen.svg)](https://docs.rs/cancel-safe-futures)
[![Documentation (main)](https://img.shields.io/badge/docs-main-brightgreen)](https://oxidecomputer.github.io/cancel-safe-futures/rustdoc/cancel_safe_futures/)
[![License](https://img.shields.io/badge/license-Apache-green.svg)](LICENSE-APACHE)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE-MIT)

Alternative futures adapters that are more cancellation-aware.

## What is this crate?

This crate solves two related but distinct problems:

### Cancel-safe futures adapters

The [`futures`](https://docs.rs/futures/latest/futures/) library contains many adapters that
make writing asynchronous Rust code more pleasant. However, some of those combinators make it
hard to write code that can withstand cancellation in the case of timeouts, `select!` branches
or similar.

For a more detailed explanation, see the documentation for `SinkExt::reserve`.

#### Example

Attempt to send an item in a loop with a timeout:

```rust
use cancel_safe_futures::prelude::*;
use std::time::Duration;

let mut my_sink = /* ... */;

// This item is stored here and will be set to None once the loop exits successfully.
let mut item = Some("hello".to_owned());
let do_flush = false;

while item.is_some() {
    match tokio::time::timeout(Duration::from_secs(10), my_sink.reserve()).await {
        Ok(Ok(permit)) => {
            let item = item.take().unwrap();
            if !do_flush {
                // permit.feed() feeds the item into the sink without flushing
                // the sink afterwards. This is a synchronous method.
                permit.feed(item)?;
            } else {
                // Alternatively, permit.send() writes the item into the sink
                // synchronously, then returns a future which can be awaited to
                // flush the sink.
                permit.send(item)?.await?;
            }
        }
        Ok(Err(error)) => return Err(error),
        Err(timeout_error) => continue,
    }
}

```

### `then_try` adapters that don't perform cancellations

The futures and tokio libraries come with a number of `try_` adapters and macros, for example
`tokio::try_join!`. These adapters have the property that if one of the futures under
consideration fails, all other futures are cancelled.

This is not always desirable and has led to correctness bugs (e.g. [omicron PR
3707](https://github.com/oxidecomputer/omicron/pull/3707)). To address this issue, this crate
provides a set of `then_try` adapters and macros that behave like their `try_` counterparts,
except that if one or more of the futures errors out, the others will still be run to
completion.

The `then_try` family includes:

* `join_then_try`: similar to `tokio::try_join`.
* `future::join_all_then_try`: similar to `futures::future::try_join_all`.
* `TryStreamExt`: contains alternative extension methods to `futures::stream::TryStreamExt`,
  such as `collect_then_try`.

#### Example

For a detailed example, see the documentation for the `join_then_try` macro.

## Notes

This library is not complete: adapters and macros are added on an as-needed basis. If you need
an adapter that is not yet implemented, please open an issue or a pull request.

## Optional features

* `macros` (enabled by default): Enables macros.
* `std` (enabled by default): Enables items that depend on `std`, including items that depend on
  `alloc`.
* `alloc` (enabled by default): Enables items that depend on `alloc`.

No-std users must turn off default features while importing this crate.

## License

This project is available under the terms of either the [Apache 2.0 license](LICENSE-APACHE) or the [MIT
license](LICENSE-MIT).

Portions derived from [futures-rs](https://github.com/rust-lang/futures-rs), and used under the
Apache 2.0 and MIT licenses.

Portions derived from [tokio](https://github.com/tokio-rs/tokio), and used under the MIT license.

<!--
README.md is generated from README.tpl by cargo readme. To regenerate:

cargo install cargo-readme
./scripts/regenerate-readmes.sh
-->
