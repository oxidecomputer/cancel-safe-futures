use core::future::Future;

// Helper function to ensure that the futures we're returning all have the right implementations.
pub(crate) fn assert_future<T, F>(future: F) -> F
where
    F: Future<Output = T>,
{
    future
}
