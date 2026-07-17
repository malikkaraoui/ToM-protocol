use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

#[cfg(feature = "lock_tracking")]
mod tracking {
    use super::*;
    use crate::{Duration, Instant};
    use std::collections::VecDeque;
    use tracing::warn;

    #[derive(Debug)]
    struct Inner<T> {
        last_lock_owner: VecDeque<(&'static str, Duration)>,
        value: T,
    }

    /// A Mutex which optionally allows to track the time a lock was held and
    /// emit warnings in case of excessive lock times
    pub(crate) struct Mutex<T> {
        inner: std::sync::Mutex<Inner<T>>,
    }

    impl<T: Debug> std::fmt::Debug for Mutex<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            std::fmt::Debug::fmt(&self.inner, f)
        }
    }

    impl<T> Mutex<T> {
        pub(crate) fn new(value: T) -> Self {
            Self {
                inner: std::sync::Mutex::new(Inner {
                    last_lock_owner: VecDeque::new(),
                    value,
                }),
            }
        }

        /// Acquires the lock for a certain purpose
        ///
        /// The purpose will be recorded in the list of last lock owners
        pub(crate) fn lock(&self, purpose: &'static str) -> MutexGuard<'_, T> {
            // We don't bother dispatching through Runtime::now because they're pure performance
            // diagnostics.
            let now = Instant::now();
            // Handle poisoned mutex gracefully: recover the value even if another thread panicked
            // while holding the lock. This prevents cascading panics in Drop impls.
            let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());

            let lock_time = Instant::now();
            let elapsed = lock_time.duration_since(now);

            if elapsed > Duration::from_millis(1) {
                warn!(
                    "Locking the connection for {} took {:?}. Last owners: {:?}",
                    purpose, elapsed, guard.last_lock_owner
                );
            }

            MutexGuard {
                guard: Some(guard),
                start_time: lock_time,
                purpose,
            }
        }
    }

    pub(crate) struct MutexGuard<'a, T> {
        // Option UNIQUEMENT pour pouvoir libérer le verrou AVANT d'émettre le
        // warn de diagnostic : logger sous verrou (subscriber lent — journald
        // NAS, pont app) transforme l'outil de mesure en source de contention.
        guard: Option<std::sync::MutexGuard<'a, Inner<T>>>,
        start_time: Instant,
        purpose: &'static str,
    }

    impl<T> Drop for MutexGuard<'_, T> {
        fn drop(&mut self) {
            let duration = self.start_time.elapsed();

            if let Some(mut guard) = self.guard.take() {
                if guard.last_lock_owner.len() == MAX_LOCK_OWNERS {
                    guard.last_lock_owner.pop_back();
                }
                guard.last_lock_owner.push_front((self.purpose, duration));
            }
            // Verrou LIBÉRÉ — le log ne peut plus retenir personne.
            if duration > Duration::from_millis(1) {
                warn!(
                    "Utilizing the connection for {} took {:?}",
                    self.purpose, duration
                );
            }
        }
    }

    impl<T> Deref for MutexGuard<'_, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            &self.guard.as_ref().expect("guard vivant hors Drop").value
        }
    }

    impl<T> DerefMut for MutexGuard<'_, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.guard.as_mut().expect("guard vivant hors Drop").value
        }
    }

    const MAX_LOCK_OWNERS: usize = 20;
}

#[cfg(feature = "lock_tracking")]
pub(crate) use tracking::Mutex;

#[cfg(not(feature = "lock_tracking"))]
mod non_tracking {
    use super::*;

    /// A Mutex which optionally allows to track the time a lock was held and
    /// emit warnings in case of excessive lock times
    #[derive(Debug)]
    pub(crate) struct Mutex<T> {
        inner: std::sync::Mutex<T>,
    }

    impl<T> Mutex<T> {
        pub(crate) fn new(value: T) -> Self {
            Self {
                inner: std::sync::Mutex::new(value),
            }
        }

        /// Acquires the lock for a certain purpose
        ///
        /// The purpose will be recorded in the list of last lock owners
        pub(crate) fn lock(&self, _purpose: &'static str) -> MutexGuard<'_, T> {
            // Handle poisoned mutex gracefully: recover the value even if another thread panicked
            // while holding the lock. This prevents cascading panics in Drop impls.
            let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            MutexGuard {
                guard,
            }
        }
    }

    pub(crate) struct MutexGuard<'a, T> {
        guard: std::sync::MutexGuard<'a, T>,
    }

    impl<T> Deref for MutexGuard<'_, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            self.guard.deref()
        }
    }

    impl<T> DerefMut for MutexGuard<'_, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.guard.deref_mut()
        }
    }
}

#[cfg(not(feature = "lock_tracking"))]
pub(crate) use non_tracking::Mutex;
