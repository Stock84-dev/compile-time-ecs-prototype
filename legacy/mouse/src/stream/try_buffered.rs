// use futures::TryStream;
// use futures::stream::{Fuse, FusedStream, FuturesOrdered, StreamExt};
// use futures_util::TryFuture;
// use futures_util::future::Fuse;
// use core::fmt;
// use core::pin::Pin;
// use futures_util::future::Future;
// use futures_util::ready;
// use futures_util::stream::Stream;
// use futures_util::task::{Context, Poll};
// use pin_project_lite::pin_project;
//
// pin_project! {
//     /// Stream for the [`buffered`](super::StreamExt::buffered) method.
//     #[must_use = "streams do nothing unless polled"]
//     pub struct TryBuffered<St>
//     where
//         St: TryStream,
//         St::Item: TryFuture,
//     {
//         #[pin]
//         stream: Fuse<St>,
//         in_progress_queue: FuturesOrdered<St::Item>,
//         max: usize,
//     }
// }
//
// impl<St> fmt::Debug for TryBuffered<St>
// where
//     St: TryStream + fmt::Debug,
//     St::Item: TryFuture,
// {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         f.debug_struct("Buffered")
//             .field("stream", &self.stream)
//             .field("in_progress_queue", &self.in_progress_queue)
//             .field("max", &self.max)
//             .finish()
//     }
// }
//
// impl<St> TryBuffered<St>
// where
//     St: TryStream,
//     St::Item: TryFuture,
// {
//     pub(super) fn new(stream: St, n: usize) -> Self {
//         Self { stream: stream.fused(), in_progress_queue: FuturesOrdered::new(), max: n }
//     }
// }
//
// impl<St> TryStream for TryBuffered<St>
// where
//     St: TryStream,
//     St::Item: TryFuture,
// {
//     type Item = <St::Item as Future>::Output;
//
//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         let mut this = self.project();
//
//         // First up, try to spawn off as many futures as possible by filling up
//         // our queue of futures.
//         while this.in_progress_queue.len() < *this.max {
//             match this.stream.as_mut().poll_next(cx) {
//                 Poll::Ready(Some(fut)) => this.in_progress_queue.push_back(fut),
//                 Poll::Ready(None) | Poll::Pending => break,
//             }
//         }
//
//         // Attempt to pull the next value from the in_progress_queue
//         let res = this.in_progress_queue.poll_next_unpin(cx);
//         if let Some(val) = futures_util::ready!(res) {
//             return Poll::Ready(Some(val));
//         }
//
//         // If more values are still coming from the stream, we're not done yet
//         if this.stream.is_done() {
//             Poll::Ready(None)
//         } else {
//             Poll::Pending
//         }
//     }
//
//     fn size_hint(&self) -> (usize, Option<usize>) {
//         let queue_len = self.in_progress_queue.len();
//         let (lower, upper) = self.stream.size_hint();
//         let lower = lower.saturating_add(queue_len);
//         let upper = match upper {
//             Some(x) => x.checked_add(queue_len),
//             None => None,
//         };
//         (lower, upper)
//     }
// }
//
// impl<St> FusedStream for TryBuffered<St>
// where
//     St: Stream,
//     St::Item: Future,
// {
//     fn is_terminated(&self) -> bool {
//         self.stream.is_done() && self.in_progress_queue.is_terminated()
//     }
// }
//
// // // Forwarding impl of Sink from the underlying stream
// // #[cfg(feature = "sink")]
// // impl<S, Item> Sink<Item> for Buffered<S>
// // where
// //     S: Stream + Sink<Item>,
// //     S::Item: Future,
// // {
// //     type Error = S::Error;
// //
// //     delegate_sink!(stream, Item);
// // }
