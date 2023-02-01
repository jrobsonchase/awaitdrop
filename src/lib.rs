//! Yet another WaitGroup implementation.
//!
//! None of the existing crates fit my needs exactly, so here's one more that
//! (hopefully) will.
//!
//! Highlights:
//! * Generalizes "tasks" to [Ref]s. More of a change in nomenclature than
//!   anything else. It's not always a group of tasks you're waiting on - it
//!   could be that you're waiting on a gaggle of structs to all be dropped.
//! * [Ref]s and [Waiter]s are entirely disjoint. You don't need a [Waiter] to
//!   create a new [Ref].
//! * Everything is cloneable and behaves as one would expect - cloned [Ref]s
//!   will all block every cloned [Waiter], which can be awaited concurrently.

#![warn(missing_docs)]

use std::{
    future::{
        Future,
        IntoFuture,
    },
    pin::Pin,
    sync::{
        self,
        Arc,
    },
    task::{
        Context,
        Poll,
        Waker,
    },
};

use futures::executor::block_on;
use parking_lot::Mutex;
use slotmap::{
    DefaultKey,
    SlotMap,
};

#[derive(Default)]
struct Wakers {
    wakers: SlotMap<DefaultKey, Option<Waker>>,
}

impl Wakers {
    fn allocate(&mut self) -> DefaultKey {
        self.wakers.insert(None)
    }

    fn insert(&mut self, idx: DefaultKey, waker: Waker) {
        if let Some(w) = self.wakers.get_mut(idx) {
            *w = Some(waker)
        }
    }

    fn remove(&mut self, idx: DefaultKey) -> Option<Waker> {
        self.wakers.remove(idx).and_then(|w| w)
    }

    fn wake_all(&mut self) {
        self.wakers
            .drain()
            .filter_map(|(_, w)| w)
            .for_each(|w| w.wake());
    }
}

/// A reference whose drop can be awaited
///
/// When cloned, creates a new reference attached to the same [Waiter].
#[derive(Clone)]
pub struct Weak {
    count: Option<sync::Weak<()>>,
    wakers: Arc<Mutex<Wakers>>,
}

impl Weak {
    /// Attempt to upgrade to a strong [Ref]
    pub fn upgrade(&self) -> Option<Ref> {
        let weak = self.count.as_ref()?;
        let strong = sync::Weak::upgrade(weak)?;

        Some(Ref {
            count: Some(strong),
            wakers: self.wakers.clone(),
        })
    }
}

/// A reference whose drop can be awaited
///
/// When cloned, creates a new reference attached to the same [Waiter].
#[derive(Clone)]
pub struct Ref {
    count: Option<Arc<()>>,
    wakers: Arc<Mutex<Wakers>>,
}

impl Ref {
    /// Get a new [Weak] that doesn't contribute to the ref count.
    pub fn downgrade(&self) -> Weak {
        let strong = self.count.as_ref().unwrap();
        let weak = Arc::downgrade(strong);
        Weak {
            count: Some(weak),
            wakers: self.wakers.clone(),
        }
    }
}

impl Drop for Ref {
    fn drop(&mut self) {
        if Arc::try_unwrap(self.count.take().unwrap()).is_ok() {
            self.wakers.lock().wake_all();
        }
    }
}

/// An awaitable handle to some number of references that will eventually be
/// dropped
#[derive(Clone)]
pub struct Waiter {
    wakers: Arc<Mutex<Wakers>>,
    count: sync::Weak<()>,
}

impl Waiter {
    /// Wait for all connected [Ref]s to be dropped in a blocking manner
    pub fn wait_blocking(&self) {
        block_on(self.wait())
    }

    /// Wait for all connected [Ref]s to be dropped
    pub fn wait(&self) -> WaitFuture {
        let idx = self.wakers.lock().allocate();
        let count = self.count.clone();
        let wakers = self.wakers.clone();
        WaitFuture { idx, wakers, count }
    }
}

/// The future returned from [Waiter::wait]
///
/// Resolves when all connected [Ref]s have been dropped.
pub struct WaitFuture {
    idx: DefaultKey,
    count: sync::Weak<()>,
    wakers: Arc<Mutex<Wakers>>,
}

impl Drop for WaitFuture {
    fn drop(&mut self) {
        self.wakers.lock().remove(self.idx);
    }
}

impl Future for WaitFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.wakers.lock().insert(self.idx, cx.waker().clone());
        if sync::Weak::strong_count(&self.count) == 0 {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl IntoFuture for Waiter {
    type IntoFuture = WaitFuture;
    type Output = ();
    fn into_future(self) -> Self::IntoFuture {
        self.wait()
    }
}

/// Create a new [Ref] and [Waiter]
///
/// The [Waiter] will resolve when the [Ref] and all clones of it have been
/// dropped.
pub fn awaitdrop() -> (Ref, Waiter) {
    let task = Ref {
        wakers: Default::default(),
        count: Some(Default::default()),
    };
    let wait = Waiter {
        count: Arc::downgrade(task.count.as_ref().unwrap()),
        wakers: task.wakers.clone(),
    };

    (task, wait)
}

#[cfg(test)]
mod test {
    use std::{
        thread,
        time::{
            self,
            Duration,
        },
    };

    use futures::executor::block_on;

    #[test]
    fn drop_wait_poll() {
        let (task, wait) = super::awaitdrop();

        drop(task);

        let fut = wait.wait();

        block_on(fut);
    }

    #[test]
    fn wait_drop_poll() {
        let (task, wait) = super::awaitdrop();

        let fut = wait.wait();

        drop(task);

        block_on(fut);
    }

    #[test]
    fn wait_poll_drop() {
        let (task, wait) = super::awaitdrop();

        let start = time::Instant::now();

        thread::spawn(move || {
            thread::sleep(Duration::from_secs(2));
            drop(task);
        });

        let fut = wait.wait();

        block_on(fut);

        assert!(time::Instant::now() - start > Duration::from_secs(2));
    }

    #[test]
    fn wait_poll_drop_lots() {
        let (task, wait) = super::awaitdrop();

        let start = time::Instant::now();

        for _ in 0..20 {
            let task = task.clone();
            thread::spawn({
                move || {
                    thread::sleep(Duration::from_secs(2));
                    drop(task);
                }
            });
        }

        drop(task);

        let fut = wait.wait();

        block_on(fut);

        assert!(time::Instant::now() - start > Duration::from_secs(2));
    }
}
