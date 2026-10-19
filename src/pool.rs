use std::{ops::Deref, sync::Arc, time::Instant};

use parking_lot::{Condvar, Mutex};

use crate::{
    config::PoolConfig,
    connection::ManageConnection,
    entry::{PoolEntry, STATE_NOT_IN_USE},
    error::{PoolError, Result},
};

pub struct Pool<M: ManageConnection> {
    pub(crate) inner: Arc<PoolInner<M>>,
}

impl<M: ManageConnection> Clone for Pool<M> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<M: ManageConnection> Pool<M> {
    pub fn new(config: PoolConfig, manager: M) -> Result<Self, M::Error> {
        if let Err(msg) = config.validate() {
            return Err(PoolError::Internal(msg.to_string()));
        }

        let inner = PoolInner {
            manager,
            config,
            entries: Mutex::new(Vec::new()),
            condvar: Condvar::new(),
        };

        Ok(Self {
            inner: Arc::new(inner),
        })
    }
}

pub(crate) struct PoolInner<M: ManageConnection> {
    pub(crate) manager: M,
    pub(crate) config: PoolConfig,
    pub(crate) entries: Mutex<Vec<Arc<PoolEntry<M::Connection>>>>,
    pub(crate) condvar: Condvar,
}

pub struct PooledConnection<M: ManageConnection> {
    pub(crate) entry: Option<Arc<PoolEntry<M::Connection>>>,
    pub(crate) pool: Arc<PoolInner<M>>,
}

impl<M: ManageConnection> Deref for PooledConnection<M> {
    type Target = M::Connection;

    fn deref(&self) -> &Self::Target {
        &self.entry.as_ref().unwrap().conn
    }
}

impl<M: ManageConnection> Drop for PooledConnection<M> {
    fn drop(&mut self) {
        if let Some(entry) = self.entry.take() {
            entry.mark_not_in_use();

            self.pool.condvar.notify_one();
        }
    }
}

impl<M: ManageConnection> Pool<M> {
    pub fn get(&self) -> Result<PooledConnection<M>, M::Error> {
        let timeout = self.inner.config.connection_timeout;
        let start = Instant::now();

        let mut entries = self.inner.entries.lock();

        loop {
            // step1: look up available connections
            for entry in entries.iter() {
                if entry.compare_and_set_in_use() {
                    return Ok(PooledConnection {
                        entry: Some(Arc::clone(entry)),
                        pool: Arc::clone(&self.inner),
                    });
                }
            }

            // step2: check if creating a new connection is available
            if entries.len() < self.inner.config.max_pool_size {
                match self.inner.manager.connect() {
                    Ok(raw_conn) => {
                        let entry = Arc::new(PoolEntry::new(raw_conn));

                        entry.compare_and_set_in_use();
                        entries.push(Arc::clone(&entry));
                        return Ok(PooledConnection {
                            entry: Some(entry),
                            pool: Arc::clone(&self.inner),
                        });
                    }
                    Err(e) => return Err(PoolError::ConnectionFailed(e)),
                }
            }

            // step3: wait until the other threads
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return Err(PoolError::Timeout(timeout));
            }
            let remaining = timeout - elapsed;

            let result = self.inner.condvar.wait_for(&mut entries, remaining);
            if result.timed_out() {
                return Err(PoolError::Timeout(timeout));
            }
        }
    }

    pub fn idle_count(&self) -> usize {
        let entries = self.inner.entries.lock();
        entries
            .iter()
            .filter(|e| e.state.load(std::sync::atomic::Ordering::Relaxed) == STATE_NOT_IN_USE)
            .count()
    }

    pub fn total_count(&self) -> usize {
        self.inner.entries.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Barrier, thread, time::Duration};

    use crate::test_helper::MockManager;

    use super::*;

    #[test]
    fn test_create_new_pool() {
        let config = PoolConfig::default();
        let manager = MockManager::default();
        assert!(Pool::new(config, manager).is_ok());
    }

    #[test]
    fn test_dref_pooled_conn() {
        let config = PoolConfig::default();
        let manager = MockManager::default();
        let pool = Pool::new(config, manager).unwrap();

        let conn = pool.get().unwrap();
        assert_eq!(conn.id, 0);

        let raw_ref = &conn;
        assert_eq!(raw_ref.id, 0);
    }

    #[test]
    fn test_drop_pooled_conn() {
        let config = PoolConfig::default();
        let manager = MockManager::default();
        let pool = Pool::new(config, manager).unwrap();

        assert_eq!(pool.total_count(), 0);
        assert_eq!(pool.idle_count(), 0);

        {
            let conn = pool.get().unwrap();
            assert_eq!(conn.id, 0);
            assert_eq!(pool.total_count(), 1);
            assert_eq!(pool.idle_count(), 0);
        }

        assert_eq!(pool.total_count(), 1);
        assert_eq!(pool.idle_count(), 1);

        let reused_conn = pool.get().unwrap();
        assert_eq!(reused_conn.id, 0);
        assert_eq!(pool.total_count(), 1);
        assert_eq!(pool.idle_count(), 0);
    }

    #[test]
    fn test_wait_conn() {
        let config = PoolConfig::default().max_pool_size(10);
        let manager = MockManager::default();
        let pool = Pool::new(config.clone(), manager).unwrap();

        let barrier = Arc::new(Barrier::new(11));
        let mut handlers = Vec::new();

        for _ in 0..10 {
            let pool_clone = pool.clone();
            let barrier_clone = Arc::clone(&barrier);

            let handler = thread::spawn(move || {
                let conn = pool_clone.get().unwrap();

                barrier_clone.wait();

                if conn.id == 5 {
                    thread::sleep(Duration::from_millis(50));
                } else {
                    thread::sleep(Duration::from_millis(300));
                }
            });
            handlers.push(handler);
        }

        barrier.wait();
        assert_eq!(pool.total_count(), config.max_pool_size);
        assert_eq!(pool.idle_count(), 0);

        let conn = pool.get().unwrap();
        assert_eq!(conn.id, 5);

        for handler in handlers {
            handler.join().unwrap();
        }
    }

    #[test]
    fn test_conn_timeout() {
        let config = PoolConfig::default()
            .max_pool_size(10)
            .connection_timeout(Duration::from_millis(10));
        let manager = MockManager::default();
        let pool = Pool::new(config.clone(), manager).unwrap();

        let barrier = Arc::new(Barrier::new(11));
        let mut handlers = Vec::new();

        for _ in 0..10 {
            let pool_clone = pool.clone();
            let barrier_clone = Arc::clone(&barrier);

            let handler = thread::spawn(move || {
                let _conn = pool_clone.get().unwrap();

                barrier_clone.wait();

                thread::sleep(Duration::from_millis(50));
            });
            handlers.push(handler);
        }

        barrier.wait();

        let result = pool.get();
        matches!(result, Err(PoolError::Timeout(_)));
    }
}
