use std::error::Error;

pub trait ManageConnection: Send + Sync + 'static {
    type Connection: Send + 'static;

    type Error: Error + Send + 'static;

    fn connect(&self) -> Result<Self::Connection, Self::Error>;

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error>;

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        let _ = conn;
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helper::MockManager;
    use std::sync::atomic::AtomicUsize;

    use super::*;

    #[test]
    fn test_mock_manager_connect() {
        let manager = MockManager {
            counter: AtomicUsize::new(0),
        };

        let conn1 = manager.connect().unwrap();
        let conn2 = manager.connect().unwrap();

        assert_eq!(conn1.id, 0);
        assert_eq!(conn2.id, 1);
    }
}
