# Changelog

## [0.1.5] - 2023-10-27

### Added

- Add `Default` and `From<T>` impls for `RobustMutex<T>`.

## [0.1.4] - 2023-10-27

- Add a `RobustMutex` which gets poisoned on panics and cancellations.

## [0.1.3] - 2023-10-27

This version was not released due to a publishing issue.

## [0.1.2] - 2023-07-27

- Add a `coop_cancel` module to enable explicit, cooperative cancellation.

## [0.1.1] - 2023-07-24

- Add support for `TryStreamExt::for_each_concurrent_then_try`.

## [0.1.0] - 2023-07-20

Initial release, with support for:

- `SinkExt::reserve`
- A `join_then_try!` macro
- A `future::join_all_then_try` adapter
- `TryStreamExt`, with a `collect_than_try` adapter

[0.1.5]: https://github.com/oxidecomputer/cancel-safe-futures/releases/cancel-safe-futures-0.1.5
[0.1.4]: https://github.com/oxidecomputer/cancel-safe-futures/releases/cancel-safe-futures-0.1.4
[0.1.3]: https://github.com/oxidecomputer/cancel-safe-futures/releases/cancel-safe-futures-0.1.3
[0.1.2]: https://github.com/oxidecomputer/cancel-safe-futures/releases/cancel-safe-futures-0.1.2
[0.1.1]: https://github.com/oxidecomputer/cancel-safe-futures/releases/cancel-safe-futures-0.1.1
[0.1.0]: https://github.com/oxidecomputer/cancel-safe-futures/releases/cancel-safe-futures-0.1.0
