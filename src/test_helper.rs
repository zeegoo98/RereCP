use std::sync::atomic::{AtomicUsize, Ordering};

use crate::connection::ManageConnection;

pub(crate) struct MockConnection {
    pub(crate) id: usize,
}

pub(crate) struct MockManager {
    pub(crate) counter: AtomicUsize,
}

impl ManageConnection for MockManager {
    type Connection = MockConnection;
    type Error = std::io::Error;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        Ok(MockConnection { id })
    }

    fn is_valid(&self, _conn: &mut Self::Connection) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Default for MockManager {
    fn default() -> Self {
        Self {
            counter: AtomicUsize::new(0),
        }
    }
}
