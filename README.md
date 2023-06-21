# cancel-safe-futures

[![Documentation (main)](https://img.shields.io/badge/docs-main-brightgreen)](https://oxidecomputer.github.io/cancel-safe-futures/rustdoc/cancel_safe_futures/)
[![License](https://img.shields.io/badge/license-Apache-green.svg)](LICENSE-APACHE)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE-MIT)

Alternative futures adapters that are more cancel-safe.

## What is this crate?

The [`futures`](https://docs.rs/futures/latest/futures/) library contains many adapters that
make writing asynchronous Rust code more pleasant. However, some of those combinators make it
hard to write code that can withstand cancellation in the case of timeouts, `select!` branches
or similar.

For a more detailed explanation, see the documentation for [`SinkExt::reserve`].

This crate contains alternative adapters that are designed for use in scenarios where
cancellation is expected.

## Example

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

## License

This project is available under the terms of either the [Apache 2.0 license](LICENSE-APACHE) or the [MIT
license](LICENSE-MIT).

Portions derived from [futures-rs](https://github.com/rust-lang/futures-rs), and used under the
Apache 2.0 and MIT licenses.

<!--
README.md is generated from README.tpl by cargo readme. To regenerate:

cargo install cargo-readme
cargo readme > README.md
-->
