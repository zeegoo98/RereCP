use std::{
    sync::atomic::{AtomicI8, Ordering},
    time::{Duration, Instant},
};

pub const STATE_NOT_IN_USE: i8 = 0;
pub const STATE_IN_USE: i8 = 1;
pub const STATE_REMOVED: i8 = -1;
pub const STATE_RESERVVED: i8 = 2;

pub struct PoolEntry<C> {
    pub conn: C,
    pub state: AtomicI8,
    pub created_at: Instant,
    pub last_used_at: Instant,
}

impl<C> PoolEntry<C> {
    pub fn new(conn: C) -> Self {
        let now = Instant::now();
        Self {
            conn,
            state: AtomicI8::new(STATE_NOT_IN_USE),
            created_at: now,
            last_used_at: now,
        }
    }

    pub fn compare_and_set(&self, expect: i8, new_state: i8) -> bool {
        self.state
            .compare_exchange(expect, new_state, Ordering::AcqRel, Ordering::Relaxed)
            .is_ok()
    }

    pub fn compare_and_set_in_use(&self) -> bool {
        self.compare_and_set(STATE_NOT_IN_USE, STATE_IN_USE)
    }

    pub fn mark_not_in_use(&self) {
        self.state.store(STATE_NOT_IN_USE, Ordering::Release);
    }

    pub fn is_expired(&self, max_lifetime: Duration) -> bool {
        self.created_at.elapsed() > max_lifetime
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{atomic::AtomicUsize, Arc, Barrier},
        thread,
    };

    use super::*;

    #[test]
    fn test_cas_state_transition() {
        let entry = PoolEntry::new("dummy_connection");
        assert_eq!(entry.state.load(Ordering::Relaxed), STATE_NOT_IN_USE);
        assert!(entry.compare_and_set_in_use());
        assert_eq!(entry.state.load(Ordering::Relaxed), STATE_IN_USE);

        entry.mark_not_in_use();
        assert!(entry.compare_and_set_in_use());
    }

    #[test]
    fn test_expired() {
        let entry = PoolEntry::new("dummy_connection");
        assert!(!entry.is_expired(Duration::from_secs(10)));
    }

    #[test]
    fn test_concurrent_cas_competition() {
        const THREAD_COUNT: usize = 50;

        let entry = Arc::new(PoolEntry::new("shared_connection"));
        let barrier = Arc::new(Barrier::new(THREAD_COUNT));

        let success_count = Arc::new(AtomicUsize::new(0));
        let failure_count = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();

        for _ in 0..THREAD_COUNT {
            let entry_clone = Arc::clone(&entry);
            let barrier_clone = Arc::clone(&barrier);
            let success_clone = Arc::clone(&success_count);
            let failure_clone = Arc::clone(&failure_count);

            let handle = thread::spawn(move || {
                barrier_clone.wait();

                if entry_clone.compare_and_set_in_use() {
                    success_clone.fetch_add(1, Ordering::SeqCst);
                } else {
                    failure_clone.fetch_add(1, Ordering::SeqCst);
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(success_count.load(Ordering::SeqCst), 1);
        assert_eq!(failure_count.load(Ordering::SeqCst), THREAD_COUNT - 1);
        assert_eq!(entry.state.load(Ordering::SeqCst), STATE_IN_USE);
    }
}
