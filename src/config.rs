use std::time::Duration;

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_pool_size: usize,
    pub min_idle: Option<usize>,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_pool_size: 10,
            min_idle: None,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(600),
            max_lifetime: Duration::from_secs(1800),
        }
    }
}

impl PoolConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_pool_size(mut self, size: usize) -> Self {
        self.max_pool_size = size;
        self
    }

    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.max_pool_size == 0 {
            return Err("max_pool_size must be greater than 0");
        }
        if let Some(min_idle) = self.min_idle && min_idle > self.max_pool_size {
            return Err("min_idle cannot be greater than max_pool_size");
        }
        if self.connection_timeout.is_zero() {
            return Err("connection_timeout must be greater than 0");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_setting() {
        let default_setting = PoolConfig::default();
        assert_eq!(default_setting.max_pool_size, 10);
        assert_eq!(default_setting.min_idle, None);
        assert_eq!(default_setting.connection_timeout, Duration::from_secs(30));
        assert_eq!(default_setting.idle_timeout, Duration::from_secs(600));
        assert_eq!(default_setting.max_lifetime, Duration::from_secs(1800));
    }

    #[test]
    fn test_update_max_pool_size() {
        let setting = PoolConfig::default().max_pool_size(100);
        assert_eq!(setting.max_pool_size, 100);
    }

    #[test]
    fn test_update_connection_timeout() {
        let setting = PoolConfig::default().connection_timeout(Duration::from_secs(60));
        assert_eq!(setting.connection_timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_validate() {
        let setting1 = PoolConfig::default().max_pool_size(0);
        assert_eq!(setting1.validate(), Err("max_pool_size must be greater than 0"));

        let setting2 = PoolConfig::default().connection_timeout(Duration::ZERO);
        assert_eq!(setting2.validate(), Err("connection_timeout must be greater than 0"));
    }
}
