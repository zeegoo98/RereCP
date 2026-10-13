use mysql::{Conn, Error, Opts};

use crate::connection::ManageConnection;

pub struct MysqlConnectionManager {
    opts: Opts,
}

impl MysqlConnectionManager {
    pub fn new<T: Into<Opts>>(opts: T) -> Self {
        Self { opts: opts.into() }
    }
}

impl ManageConnection for MysqlConnectionManager {
    type Connection = Conn;
    type Error = Error;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        Conn::new(self.opts.clone())
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        conn.ping()
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        conn.ping().is_err()
    }
}

#[cfg(test)]
mod tests {
    use mysql::{Conn, OptsBuilder};

    use crate::{connection::ManageConnection, connection_mysql::MysqlConnectionManager};

    #[test]
    #[ignore]
    fn test_real_mysql_connection() {
        let opts = OptsBuilder::new()
            .ip_or_hostname(Some("localhost"))
            .tcp_port(3306)
            .user(Some("root"))
            .pass(Some(""));
        let manager = MysqlConnectionManager::new(opts);

        let mut conn: Conn = manager.connect().expect("MySQL connection failed");

        assert!(manager.is_valid(&mut conn).is_ok());

        use mysql::prelude::*;
        let result: Vec<i32> = conn.query("SELECT 1").unwrap();
        assert_eq!(result, vec![1]);
    }
}
