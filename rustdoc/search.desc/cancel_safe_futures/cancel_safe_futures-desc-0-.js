searchState.loadedDescShard("cancel_safe_futures", 0, "Alternative futures adapters that are more …\nA multi-producer, single-consumer channel for cooperative …\nAlternative adapters for asynchronous values.\nWaits on multiple concurrent branches for <strong>all</strong> futures to …\nA “prelude” for crates using the <code>cancel-safe-futures</code> …\nAlternative extensions for <code>Sink</code>.\nAlternative adapters for asynchronous streams.\nAlternative synchronization primitives that are more …\nA cooperative cancellation sender.\nA cooperative cancellation receiver.\nA future which can be used to optionally block until a …\nPerforms a cancellation with a message.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCreates and returns a cooperative cancellation pair.\nReceives a cancellation payload, or <code>None</code> if either:\nFuture for the <code>join_all_then_try</code> function.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCreates a future which represents either a collection of …\nFuture for the <code>flush_reserve</code> method.\nA permit to send an item to a sink.\nFuture for the <code>reserve</code> method.\nExtension trait for <code>Sink</code> that provides alternative …\nSends an item to the sink, akin to the <code>SinkExt::feed</code> …\nA future that completes once the sink is flushed, and an …\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nA future that completes once an item is ready to be sent …\nSends an item to the sink and then flushes it, akin to the …\nFuture for the <code>collect_then_try</code> method.\nFuture for the <code>for_each_concurrent_then_try</code> method.\nAlternative adapters for <code>Result</code>-returning streams\nAttempt to transform a stream into a collection, returning …\nAttempt to transform a stream into a collection, returning …\nAttempts to run this stream to completion, executing the …\nAttempts to run this stream to completion, executing the …\nReturns the argument unchanged.\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nA token that grants the ability to run one closure against …\nContains the error value\nContains the error value\nA type alias for the result of a lock method which can be …\nContains the success value\nContains the success value\nA type of error which can be returned whenever a …\nThe lock could not be acquired because another task failed …\nA cancel-safe and panic-safe variant of <code>tokio::sync::Mutex</code>.\nAn enumeration of possible errors associated with a …\nA type alias for the result of a nonblocking locking …\nThe lock could not be acquired at this time because the …\nBlockingly locks this <code>Mutex</code>. When the lock has been …\nCreates a new lock in an unlocked state ready for use.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns a mutable reference to the underlying data.\nReaches into this error indicating that a lock is …\nReaches into this error indicating that a lock is …\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nConsumes the mutex, returning the underlying data.\nConsumes this error indicating that a lock is poisoned, …\nReturns true if this error indicates that the lock was …\nDetermines whether this mutex is poisoned due to a …\nReturns true if this error indicates that the lock was …\nDetermines whether the mutex is poisoned due to a panic.\nDetermines whether the mutex is poisoned.\nLocks this mutex, causing the current task to yield until …\nCreates a new lock in an unlocked state ready for use.\nRuns a closure with access to the guarded data, consuming …\nRuns an asynchronous block in the context of the guarded …\nRuns a non-<code>Send</code> asynchronous block in the context of the …\nAttempts to acquire the lock, returning an <code>ActionPermit</code> if …")