use anyhow::{Context, Result};
use std::env;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct Config {
    pub mqtt: MqttConfig,
    pub cityflitzer: CityflitzerConfig,
    pub general: GeneralConfig,
}

#[derive(Debug, Clone)]
pub struct MqttConfig {
    pub host: String,
    pub username: String,
    pub password: String,
    pub connect_timeout_ms: u64,
    pub client_id: String,
}

#[derive(Debug, Clone)]
pub struct CityflitzerConfig {
    pub api_key: String,
    pub latitude: f64,
    pub longitude: f64,
    pub max_distance: f64,
    pub search_range: u32,
    pub base_url: String,
}

#[derive(Debug, Clone)]
pub struct GeneralConfig {
    pub refresh_interval_secs: u64,
    pub http_timeout_ms: u64,
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            mqtt: MqttConfig::from_env()?,
            cityflitzer: CityflitzerConfig::from_env()?,
            general: GeneralConfig::from_env()?,
        })
    }
}

impl MqttConfig {
    fn from_env() -> Result<Self> {
        Ok(MqttConfig {
            host: env::var("MQTT_HOST").unwrap_or_else(|_| "tcp://10.0.0.9:1883".to_string()),
            username: env::var("MQTT_USERNAME")
                .context("MQTT_USERNAME environment variable is required")?,
            password: env::var("MQTT_PASSWORD")
                .context("MQTT_PASSWORD environment variable is required")?,
            connect_timeout_ms: parse_env_var("MQTT_CONNECT_TIMEOUT_MS", 100)?,
            client_id: env::var("MQTT_CLIENT_ID").unwrap_or_else(|_| "mqttbot".to_string()),
        })
    }

    pub fn connect_timeout(&self) -> Duration {
        Duration::from_millis(self.connect_timeout_ms)
    }
}

impl CityflitzerConfig {
    fn from_env() -> Result<Self> {
        Ok(CityflitzerConfig {
            api_key: env::var("CITYFLITZER_API_KEY")
                .unwrap_or_else(|_| "45d38969-0086-978d-dc06-7959b0d2fe79".to_string()),
            latitude: parse_env_var("CITYFLITZER_LATITUDE", 51.32032033409821)?,
            longitude: parse_env_var("CITYFLITZER_LONGITUDE", 12.36535400104385)?,
            max_distance: parse_env_var("CITYFLITZER_MAX_DISTANCE", 500.0)?,
            search_range: parse_env_var("CITYFLITZER_SEARCH_RANGE", 30000)?,
            base_url: env::var("CITYFLITZER_BASE_URL")
                .unwrap_or_else(|_| "https://de1.cantamen.de/casirest/v3".to_string()),
        })
    }
}

impl GeneralConfig {
    fn from_env() -> Result<Self> {
        Ok(GeneralConfig {
            refresh_interval_secs: parse_env_var("REFRESH_INTERVAL", 120)?,
            http_timeout_ms: parse_env_var("HTTP_TIMEOUT_MS", 5000)?,
            max_retries: parse_env_var("MAX_RETRIES", 10)?,
            initial_backoff_ms: parse_env_var("INITIAL_BACKOFF_MS", 10000)?,
        })
    }

    pub fn refresh_interval(&self) -> Duration {
        Duration::from_secs(self.refresh_interval_secs)
    }

    pub fn http_timeout(&self) -> Duration {
        Duration::from_millis(self.http_timeout_ms)
    }

    pub fn initial_backoff(&self) -> Duration {
        Duration::from_millis(self.initial_backoff_ms)
    }
}

fn parse_env_var<T>(var_name: &str, default: T) -> Result<T>
where
    T: std::str::FromStr + std::fmt::Display,
    T::Err: std::fmt::Display,
{
    match env::var(var_name) {
        Ok(value) => value.parse().map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse environment variable {}: '{}' - {}",
                var_name,
                value,
                e
            )
        }),
        Err(_) => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_env_var_with_default() {
        // Test that default value is returned when env var is not set
        let result: Result<u64> = parse_env_var("NON_EXISTENT_VAR", 42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_config_durations() {
        let general = GeneralConfig {
            refresh_interval_secs: 120,
            http_timeout_ms: 5000,
            max_retries: 10,
            initial_backoff_ms: 10000,
        };

        assert_eq!(general.refresh_interval(), Duration::from_secs(120));
        assert_eq!(general.http_timeout(), Duration::from_millis(5000));
        assert_eq!(general.initial_backoff(), Duration::from_millis(10000));
    }
}
