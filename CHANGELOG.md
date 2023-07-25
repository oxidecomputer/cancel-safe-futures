# Changelog

## [0.1.1] - 2023-07-24

- Add support for `TryStreamExt::for_each_concurrent_then_try`.

## [0.1.0] - 2023-07-20

Initial release, with support for:
- `SinkExt::reserve`
- A `join_then_try!` macro
- A `future::join_all_then_try` adapter
- `TryStreamExt`, with a `collect_than_try` adapter

[0.1.1]: https://github.com/oxidecomputer/cancel-safe-futures/releases/cancel-safe-futures-0.1.1
[0.1.0]: https://github.com/oxidecomputer/cancel-safe-futures/releases/cancel-safe-futures-0.1.0
