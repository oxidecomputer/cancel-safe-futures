(function() {var implementors = {
"bytes":[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.81.0/std/primitive.u8.html\">u8</a>&gt; for <a class=\"struct\" href=\"bytes/struct.Bytes.html\" title=\"struct bytes::Bytes\">Bytes</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.81.0/std/primitive.u8.html\">u8</a>&gt; for <a class=\"struct\" href=\"bytes/struct.BytesMut.html\" title=\"struct bytes::BytesMut\">BytesMut</a>"],["impl&lt;'a&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;&amp;'a <a class=\"primitive\" href=\"https://doc.rust-lang.org/1.81.0/std/primitive.u8.html\">u8</a>&gt; for <a class=\"struct\" href=\"bytes/struct.BytesMut.html\" title=\"struct bytes::BytesMut\">BytesMut</a>"]],
"cancel_safe_futures":[["impl&lt;F&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;F&gt; for <a class=\"struct\" href=\"cancel_safe_futures/future/struct.JoinAllThenTry.html\" title=\"struct cancel_safe_futures::future::JoinAllThenTry\">JoinAllThenTry</a>&lt;F&gt;<div class=\"where\">where\n    F: <a class=\"trait\" href=\"futures_core/future/trait.TryFuture.html\" title=\"trait futures_core::future::TryFuture\">TryFuture</a>,</div>"]],
"futures_util":[["impl&lt;F&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;F&gt; for <a class=\"struct\" href=\"futures_util/future/struct.TryJoinAll.html\" title=\"struct futures_util::future::TryJoinAll\">TryJoinAll</a>&lt;F&gt;<div class=\"where\">where\n    F: <a class=\"trait\" href=\"futures_util/future/trait.TryFuture.html\" title=\"trait futures_util::future::TryFuture\">TryFuture</a>,</div>"],["impl&lt;F: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;F&gt; for <a class=\"struct\" href=\"futures_util/future/struct.JoinAll.html\" title=\"struct futures_util::future::JoinAll\">JoinAll</a>&lt;F&gt;"],["impl&lt;Fut&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;Fut&gt; for <a class=\"struct\" href=\"futures_util/stream/struct.FuturesUnordered.html\" title=\"struct futures_util::stream::FuturesUnordered\">FuturesUnordered</a>&lt;Fut&gt;"],["impl&lt;Fut: <a class=\"trait\" href=\"futures_util/future/trait.TryFuture.html\" title=\"trait futures_util::future::TryFuture\">TryFuture</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;Fut&gt; for <a class=\"struct\" href=\"futures_util/future/struct.SelectOk.html\" title=\"struct futures_util::future::SelectOk\">SelectOk</a>&lt;Fut&gt;"],["impl&lt;Fut: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;Fut&gt; for <a class=\"struct\" href=\"futures_util/future/struct.SelectAll.html\" title=\"struct futures_util::future::SelectAll\">SelectAll</a>&lt;Fut&gt;"],["impl&lt;Fut: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;Fut&gt; for <a class=\"struct\" href=\"futures_util/stream/struct.FuturesOrdered.html\" title=\"struct futures_util::stream::FuturesOrdered\">FuturesOrdered</a>&lt;Fut&gt;"],["impl&lt;St: <a class=\"trait\" href=\"futures_util/stream/trait.Stream.html\" title=\"trait futures_util::stream::Stream\">Stream</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/marker/trait.Unpin.html\" title=\"trait core::marker::Unpin\">Unpin</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;St&gt; for <a class=\"struct\" href=\"futures_util/stream/struct.SelectAll.html\" title=\"struct futures_util::stream::SelectAll\">SelectAll</a>&lt;St&gt;"]],
"proc_macro2":[["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;<a class=\"enum\" href=\"proc_macro2/enum.TokenTree.html\" title=\"enum proc_macro2::TokenTree\">TokenTree</a>&gt; for <a class=\"struct\" href=\"proc_macro2/struct.TokenStream.html\" title=\"struct proc_macro2::TokenStream\">TokenStream</a>"],["impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;<a class=\"struct\" href=\"proc_macro2/struct.TokenStream.html\" title=\"struct proc_macro2::TokenStream\">TokenStream</a>&gt; for <a class=\"struct\" href=\"proc_macro2/struct.TokenStream.html\" title=\"struct proc_macro2::TokenStream\">TokenStream</a>"]],
"slab":[["impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;(<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.81.0/std/primitive.usize.html\">usize</a>, T)&gt; for <a class=\"struct\" href=\"slab/struct.Slab.html\" title=\"struct slab::Slab\">Slab</a>&lt;T&gt;"]],
"smallvec":[["impl&lt;A: <a class=\"trait\" href=\"smallvec/trait.Array.html\" title=\"trait smallvec::Array\">Array</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;&lt;A as <a class=\"trait\" href=\"smallvec/trait.Array.html\" title=\"trait smallvec::Array\">Array</a>&gt;::<a class=\"associatedtype\" href=\"smallvec/trait.Array.html#associatedtype.Item\" title=\"type smallvec::Array::Item\">Item</a>&gt; for <a class=\"struct\" href=\"smallvec/struct.SmallVec.html\" title=\"struct smallvec::SmallVec\">SmallVec</a>&lt;A&gt;"]],
"syn":[["impl&lt;T, P&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;<a class=\"enum\" href=\"syn/punctuated/enum.Pair.html\" title=\"enum syn::punctuated::Pair\">Pair</a>&lt;T, P&gt;&gt; for <a class=\"struct\" href=\"syn/punctuated/struct.Punctuated.html\" title=\"struct syn::punctuated::Punctuated\">Punctuated</a>&lt;T, P&gt;"],["impl&lt;T, P&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;T&gt; for <a class=\"struct\" href=\"syn/punctuated/struct.Punctuated.html\" title=\"struct syn::punctuated::Punctuated\">Punctuated</a>&lt;T, P&gt;<div class=\"where\">where\n    P: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/default/trait.Default.html\" title=\"trait core::default::Default\">Default</a>,</div>"]],
"tokio":[["impl&lt;T, F&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/iter/traits/collect/trait.FromIterator.html\" title=\"trait core::iter::traits::collect::FromIterator\">FromIterator</a>&lt;F&gt; for <a class=\"struct\" href=\"tokio/task/struct.JoinSet.html\" title=\"struct tokio::task::JoinSet\">JoinSet</a>&lt;T&gt;<div class=\"where\">where\n    F: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/future/future/trait.Future.html\" title=\"trait core::future::future::Future\">Future</a>&lt;Output = T&gt; + <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'static,\n    T: <a class=\"trait\" href=\"https://doc.rust-lang.org/1.81.0/core/marker/trait.Send.html\" title=\"trait core::marker::Send\">Send</a> + 'static,</div>"]]
};if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()