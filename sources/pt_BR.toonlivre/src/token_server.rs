use aidoku::alloc::String;
use aidoku::prelude::*;
use serde::{Deserialize, Serialize};

const TOKEN_SERVER_CONFIG: &str = include_str!("../res/token-server.json");

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenServerConfig {
	pub schema_version: i64,
	pub enabled: bool,
	pub host: String,
	pub endpoints: TokenServerEndpoints,
	pub timeout: TokenServerTimeout,
	pub cache: TokenServerCache,
	pub fallback: TokenServerFallback,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenServerEndpoints {
	pub tokens: String,
	pub health: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenServerTimeout {
	pub connect: i32,
	pub request: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenServerCache {
	pub enabled: bool,
	pub ttl: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenServerFallback {
	pub enabled: bool,
	pub retry_count: i32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenServerHeaders {
	#[serde(rename = "x-toon-signature")]
	pub signature: String,
	#[serde(rename = "x-toon-verify")]
	pub verify: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct TokenServerResponse {
	pub session: Option<String>,
	pub passphrase: Option<String>,
	pub headers: Option<TokenServerHeaders>,
	pub strategy: Option<String>,
	#[serde(default, rename = "expiresAt")]
	pub expires_at: Option<i64>,
	#[serde(default)]
	pub cached: bool,
	#[serde(default, rename = "expiresIn")]
	pub expires_in: i32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct DecryptResponse {
	pub decrypted: String,
}

pub fn load_token_server_config() -> Option<TokenServerConfig> {
	let config: TokenServerConfig = serde_json::from_str(TOKEN_SERVER_CONFIG).ok()?;
	Some(config)
}

#[allow(dead_code)]
pub fn token_server_enabled() -> bool {
	load_token_server_config()
		.map(|config| config.enabled)
		.unwrap_or(false)
}

pub fn token_server_url(endpoint: &str) -> Option<String> {
	let config = load_token_server_config()?;

	// Override host for dev-server feature
	#[cfg(feature = "dev-server")]
	let host = String::from("http://localhost:3000");
	#[cfg(not(feature = "dev-server"))]
	let host = config.host;

	let base = host.trim_end_matches('/');
	let path = endpoint.trim_start_matches('/');
	Some(format!("{}/{}", base, path))
}

pub fn full_tokens_url() -> Option<String> {
	let config = load_token_server_config()?;
	token_server_url(&config.endpoints.tokens)
}

#[allow(dead_code)]
pub fn full_health_url() -> Option<String> {
	let config = load_token_server_config()?;
	token_server_url(&config.endpoints.health)
}

#[cfg(test)]
mod token_server_tests {
	use super::*;
	use aidoku_test::aidoku_test;

	fn get_expected_host() -> &'static str {
		if cfg!(feature = "dev-server") {
			"http://localhost:3000"
		} else {
			"https://toons.4nd.xyz"
		}
	}

	#[aidoku_test]
	fn config_loads_successfully() {
		let config = load_token_server_config();
		assert!(config.is_some());
	}

	#[aidoku_test]
	fn config_has_valid_schema_version() {
		let config = load_token_server_config().expect("Config should load");
		assert_eq!(config.schema_version, 1);
	}

	#[aidoku_test]
	fn config_is_enabled() {
		let config = load_token_server_config().expect("Config should load");
		assert!(config.enabled);
	}

	#[aidoku_test]
	fn config_has_valid_host() {
		let config = load_token_server_config().expect("Config should load");
		assert!(!config.host.is_empty());
		assert!(config.host.starts_with("http://") || config.host.starts_with("https://"));
		// Config always loads from the file, which has production URL
		assert_eq!(config.host, "https://toons.4nd.xyz");
	}

	#[aidoku_test]
	fn effective_host_respects_dev_server_feature() {
		// The effective host used in URLs is overridden by dev-server feature
		let url = full_tokens_url().expect("Should generate tokens URL");
		assert_eq!(url, format!("{}/api/tokens", get_expected_host()));
	}

	#[aidoku_test]
	fn config_has_all_required_endpoints() {
		let config = load_token_server_config().expect("Config should load");
		assert_eq!(config.endpoints.tokens, "/api/tokens");
		assert_eq!(config.endpoints.health, "/health");
	}

	#[aidoku_test]
	fn config_has_valid_timeouts() {
		let config = load_token_server_config().expect("Config should load");
		assert!(config.timeout.connect > 0);
		assert!(config.timeout.request > 0);
		assert!(config.timeout.connect <= 60);
		assert!(config.timeout.request <= 120);
	}

	#[aidoku_test]
	fn config_has_valid_cache_settings() {
		let config = load_token_server_config().expect("Config should load");
		assert!(config.cache.enabled);
		assert!(config.cache.ttl > 0);
		assert!(config.cache.ttl <= 300);
	}

	#[aidoku_test]
	fn config_has_valid_fallback_settings() {
		let config = load_token_server_config().expect("Config should load");
		assert!(config.fallback.retry_count >= 0);
		assert!(config.fallback.retry_count <= 5);
	}

	#[aidoku_test]
	fn token_server_enabled_returns_correct_value() {
		assert!(token_server_enabled());
	}

	#[aidoku_test]
	fn full_tokens_url_generates_correct_url() {
		let url = full_tokens_url().expect("Should generate tokens URL");
		assert_eq!(url, format!("{}/api/tokens", get_expected_host()));
	}

	#[aidoku_test]
	fn full_health_url_generates_correct_url() {
		let url = full_health_url().expect("Should generate health URL");
		assert_eq!(url, format!("{}/health", get_expected_host()));
	}

	#[aidoku_test]
	fn token_server_url_handles_leading_slash() {
		let url = token_server_url("/api/tokens").expect("Should handle leading slash");
		assert_eq!(url, format!("{}/api/tokens", get_expected_host()));
	}

	#[aidoku_test]
	fn token_server_url_handles_no_leading_slash() {
		let url = token_server_url("api/tokens").expect("Should handle no leading slash");
		assert_eq!(url, format!("{}/api/tokens", get_expected_host()));
	}

	#[aidoku_test]
	fn token_server_url_strips_trailing_slash_from_host() {
		let config = load_token_server_config().expect("Config should load");
		assert!(!config.host.ends_with('/'));
	}
}
