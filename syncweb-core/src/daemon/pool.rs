use std::fmt;

use crate::error::{Result, SyncwebError};

/// A fixed-size Rayon pool owned by a daemon for CPU-bound archive work.
pub struct ManagedPool {
    pool: rayon::ThreadPool,
    name: String,
    thread_count: usize,
}

impl ManagedPool {
    /// Create a named pool with a fixed number of worker threads.
    ///
    /// A thread count of zero uses the host's available parallelism.
    ///
    /// # Errors
    ///
    /// Returns an error when Rayon cannot create the pool.
    pub fn new(name: impl Into<String>, thread_count: usize) -> Result<Self> {
        let pool_name = name.into();
        let normalized_threads = if thread_count == 0 {
            std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
        } else {
            thread_count
        };
        let thread_name = pool_name.clone();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(normalized_threads)
            .thread_name(move |index| format!("{thread_name}-{index}"))
            .build()
            .map_err(|error| SyncwebError::operation("failed to create managed thread pool", error))?;
        Ok(Self {
            pool,
            name: pool_name,
            thread_count: normalized_threads,
        })
    }

    /// Run a synchronous operation on this pool and return its result.
    pub fn install<F, R>(&self, operation: F) -> R
    where
        F: FnOnce() -> R + Send,
        R: Send,
    {
        self.pool.install(operation)
    }

    /// Queue a task in FIFO order on this pool.
    pub fn spawn_fifo<F>(&self, operation: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.pool.spawn_fifo(operation);
    }

    /// Return the fixed number of worker threads in this pool.
    #[must_use]
    pub const fn thread_count(&self) -> usize {
        self.thread_count
    }
}

impl fmt::Debug for ManagedPool {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ManagedPool")
            .field("name", &self.name)
            .field("thread_count", &self.thread_count)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, thread, time::Duration};

    use super::ManagedPool;

    #[test]
    fn custom_thread_count_is_fixed() -> crate::error::Result<()> {
        let pool = ManagedPool::new("test-archive", 2)?;
        if pool.thread_count() != 2 {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "managed pool did not retain its configured thread count".to_owned(),
            ));
        }
        Ok(())
    }

    #[test]
    fn install_runs_on_named_pool_thread() -> crate::error::Result<()> {
        let pool = ManagedPool::new("test-archive", 1)?;
        let worker_name_result = pool.install(|| thread::current().name().map(str::to_owned));
        let worker_name = worker_name_result
            .ok_or_else(|| crate::error::SyncwebError::InvalidConfig("managed pool thread has no name".to_owned()))?;
        if !worker_name.starts_with("test-archive-") {
            return Err(crate::error::SyncwebError::InvalidConfig(
                "managed pool worker name has an unexpected prefix".to_owned(),
            ));
        }
        Ok(())
    }

    #[test]
    fn spawn_fifo_executes() -> crate::error::Result<()> {
        let pool = ManagedPool::new("test-archive", 1)?;
        let (sender, receiver) = mpsc::channel();
        pool.spawn_fifo(move || {
            let _ = sender.send(());
        });
        receiver
            .recv_timeout(Duration::from_secs(1))
            .map_err(|error| crate::error::SyncwebError::operation("managed pool task did not run", error))?;
        Ok(())
    }
}
