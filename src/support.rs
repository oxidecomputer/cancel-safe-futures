use core::future::Future;

// Helper function to ensure that the futures we're returning all have the right implementations.
pub(crate) fn assert_future<T, F>(future: F) -> F
where
    F: Future<Output = T>,
{
    future
}

// From https://twitter.com/8051Enthusiast/status/1571909110009921538
extern "C" {
    fn __cancel_safe_external_symbol_that_doesnt_exist();
}

#[inline]
#[allow(dead_code)]
pub(crate) fn statically_unreachable() -> ! {
    unsafe {
        __cancel_safe_external_symbol_that_doesnt_exist();
    }
    unreachable!("linker symbol above cannot be resolved")
}
