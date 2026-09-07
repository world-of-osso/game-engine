use super::TaskPoolBuilder;
use core::{future::poll_fn, task::Poll};
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn bulk_submission_returns_borrowed_results_and_accepts_empty_input() {
    let pool = TaskPoolBuilder::new().num_threads(2).build();
    let values = [3, 7, 11, 19];
    let mut outputs = pool.scope(|scope| {
        scope.spawn_many(values.iter().map(|value| async move { value * 2 }));
    });
    outputs.sort_unstable();
    assert_eq!(outputs, [6, 14, 22, 38]);

    let outputs = pool.scope(|scope| {
        scope.spawn_many(values[..0].iter().map(|value| async move { value * 2 }));
    });
    assert!(outputs.is_empty());
}

#[test]
fn bulk_submission_polls_sibling_futures_independently() {
    let pool = TaskPoolBuilder::new().num_threads(1).build();
    let (sender, receiver) = async_channel::bounded(1);
    let outputs = pool.scope(|scope| {
        scope.spawn_many((0..2).map(|index| {
            let sender = &sender;
            let receiver = &receiver;
            async move {
                if index == 0 {
                    receiver.recv().await.unwrap()
                } else {
                    sender.send(41).await.unwrap();
                    1
                }
            }
        }));
    });
    assert_eq!(outputs.len(), 2);
    assert_eq!(outputs.iter().sum::<i32>(), 42);
}

struct CountDrop<'a>(&'a AtomicUsize);

impl Drop for CountDrop<'_> {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn bulk_submission_propagates_panic_and_drains_borrowed_futures() {
    let pool = TaskPoolBuilder::new().num_threads(1).build();
    let dropped = AtomicUsize::new(0);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pool.scope(|scope| {
            scope.spawn_many((0..4).map(|index| {
                let guard = CountDrop(&dropped);
                async move {
                    let _guard = guard;
                    if index == 0 {
                        panic!("bulk task panic");
                    }
                    poll_fn(|_| Poll::<()>::Pending).await;
                }
            }));
        });
    }));
    let payload = result.expect_err("task panic must propagate out of scope");
    assert_eq!(payload.downcast_ref::<&str>(), Some(&"bulk task panic"));
    assert_eq!(dropped.load(Ordering::SeqCst), 4);

    let results = pool.scope(|scope| scope.spawn_many([async { 23 }]));
    assert_eq!(results, [23]);
}
