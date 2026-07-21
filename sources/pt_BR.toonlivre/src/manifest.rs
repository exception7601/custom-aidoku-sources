use aidoku::prelude::*;
use aidoku::{
	alloc::{String, Vec, format, vec},
	imports::{
		defaults::{DefaultValue, defaults_get, defaults_set},
		net::Request,
		std::current_date,
	},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::{ACCEPT_LANGUAGE, BASE_URL};

const BUNDLED_MANIFEST_JSON: &str = include_str!("../res/manifest.json");
const REMOTE_MANIFEST_URL: &str =
	"https://exception7601.github.io/custom-aidoku-sources/manifest.json";
const MANIFEST_REQUEST_COUNTER_KEY: &str = "toonlivre.manifest.request-counter";
const MANIFEST_SEED_TOKEN_KEY: &str = "toonlivre.manifest.seed-token";
const MANIFEST_SOURCE_ID: &str = "pt_BR.toonlivre";
const FALLBACK_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/127.0.0.0 Safari/537.36";
const FALLBACK_SIGNATURE_HEADER: &str = "x-toon-signature";
const FALLBACK_DATA_KEY_HEADER: &str = "x-toon-datakey";
const FALLBACK_COOKIE_NAME: &str = "toon_v";
const FALLBACK_SEED_META_NAME: &str = "t-seed";
const FALLBACK_SEED_ENDPOINT_PATH: &str = "/api/seed";
const FALLBACK_SEED_TOKEN_FIELD: &str = "token";
const FALLBACK_DECRYPTION_PREFIX: &str = "Magnesium-Strike-Astonish3";
const FALLBACK_DECRYPTION_SALT: &str = "toonlivre.com::v8";
const FALLBACK_DECRYPTION_SUFFIX: &str = "t8_4v2_b";
const SEED_TOKEN_EXPIRY_MARGIN_MS: i64 = 120_000;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientManifest {
	pub schema_version: i64,
	pub source_id: String,
	pub site_url: String,
	pub request: ManifestRequest,
	pub decrypt: DecryptManifest,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManifestRequest {
	pub user_agent: String,
	pub accept_language: String,
	pub signature_header: String,
	#[serde(default)]
	pub signature_rules: Vec<ManifestSignatureRule>,
	#[serde(default, rename = "signatureStrategy")]
	pub signature_strategy: Option<ManifestSignatureStrategy>,
	#[serde(default)]
	pub verify_header: Option<String>,
	pub include_credentials: bool,
	pub session_cookie: ManifestSessionCookie,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManifestSignatureRule {
	pub value: String,
	#[serde(default)]
	pub default: bool,
	#[serde(default)]
	pub when: Option<ManifestSignatureMatch>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManifestSignatureMatch {
	pub url_contains: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum ManifestSignatureStrategy {
	#[serde(rename = "time-sha256-base64")]
	TimeSha256Base64 {
		#[serde(rename = "timestampDivisor")]
		timestamp_divisor: i64,
		salt: String,
		#[serde(rename = "routeSelector")]
		route_selector: ManifestRouteSelector,
	},
	#[serde(rename = "seed-jwt")]
	SeedJwt {
		#[serde(rename = "metaName")]
		meta_name: String,
		#[serde(rename = "endpointPath")]
		endpoint_path: String,
		#[serde(rename = "tokenField")]
		token_field: String,
	},
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManifestRouteSelector {
	pub when_url_contains: String,
	pub when_matched: String,
	pub otherwise: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManifestSessionCookie {
	pub name: String,
	pub generator: ManifestSessionCookieGenerator,
	#[serde(default)]
	pub mirrors_into: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub(crate) enum ManifestSessionCookieGenerator {
	#[serde(rename = "random-base36-concat")]
	RandomBase36Concat {
		segments: Vec<ManifestRandomSegment>,
	},
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ManifestRandomSegment {
	pub radix: u8,
	pub start: usize,
	pub end: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DecryptManifest {
	pub data_key_header: String,
	pub payload_selector: String,
	pub algorithm: String,
	pub passphrase: ManifestPassphraseStrategy,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum ManifestPassphraseStrategy {
	#[serde(rename = "utc-md5-derived")]
	UtcMd5Derived {
		#[serde(rename = "dateFormat")]
		date_format: String,
		prefix: String,
		salt: String,
		suffix: String,
		#[serde(rename = "digestEncoding")]
		digest_encoding: String,
		#[serde(rename = "digestSlice")]
		digest_slice: ManifestDigestSlice,
	},
	#[serde(rename = "utc-sha256-derived")]
	UtcSha256Derived {
		#[serde(rename = "dateFormat")]
		date_format: String,
		prefix: String,
		salt: String,
		suffix: String,
		#[serde(rename = "digestEncoding")]
		digest_encoding: String,
		#[serde(rename = "digestSlice")]
		digest_slice: ManifestDigestSlice,
	},
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManifestDigestSlice {
	pub start: usize,
	pub end: usize,
}

pub(crate) fn active_manifest() -> ClientManifest {
	let bundled = bundled_manifest();
	if let Some(remote) = fetch_remote_manifest() {
		let remote_score = manifest_capability_score(&remote);
		let bundled_score = manifest_capability_score(&bundled);

		if remote_score >= bundled_score {
			source_log!(
				"[toonlivre] active_manifest source=remote remote_score={} bundled_score={}",
				remote_score,
				bundled_score
			);
			return remote;
		}

		source_log!(
			"[toonlivre] active_manifest source=local_fallback_stale_remote remote_score={} bundled_score={}",
			remote_score,
			bundled_score
		);
		return bundled;
	}
	source_log!("[toonlivre] active_manifest source=local_fallback");
	bundled
}

pub(crate) fn bundled_manifest() -> ClientManifest {
	parse_manifest(BUNDLED_MANIFEST_JSON).unwrap_or_else(default_manifest)
}

pub(crate) fn request_verification_token() -> String {
	generate_session_cookie_value(&bundled_manifest())
}

pub(crate) fn current_decryption_passphrase() -> String {
	current_decryption_passphrase_for_manifest(&bundled_manifest())
}

pub(crate) fn signature_value_for_url(manifest: &ClientManifest, url: &str) -> String {
	let session_token = generate_session_cookie_value(manifest);
	signature_value_for_url_with_session(manifest, url, &session_token)
}

pub(crate) fn signature_value_for_url_with_session(
	manifest: &ClientManifest,
	url: &str,
	session_token: &str,
) -> String {
	if let Some(strategy) = manifest.request.signature_strategy.as_ref() {
		return dynamic_signature_value(manifest, strategy, url, session_token);
	}

	select_static_signature_value(&manifest.request.signature_rules, url)
}

pub(crate) fn token_mirror_header_names(manifest: &ClientManifest) -> Vec<String> {
	let mut header_names = Vec::new();

	if let Some(verify_header) = manifest.request.verify_header.as_ref() {
		if !verify_header.trim().is_empty() {
			header_names.push(verify_header.clone());
		}
	}

	for header_name in manifest.request.session_cookie.mirrors_into.iter() {
		if header_name.trim().is_empty()
			|| header_names.iter().any(|existing| existing == header_name)
		{
			continue;
		}

		header_names.push(header_name.clone());
	}

	header_names
}

pub(crate) fn describe_token_mirror(manifest: &ClientManifest) -> String {
	let mut parts = token_mirror_header_names(manifest);
	parts.push(format!("cookie {}", manifest.request.session_cookie.name));

	let mut description = String::new();
	for (index, part) in parts.iter().enumerate() {
		if index > 0 {
			description.push_str(" + ");
		}
		description.push_str(part.as_str());
	}

	description
}

pub(crate) fn generate_session_cookie_value(manifest: &ClientManifest) -> String {
	match &manifest.request.session_cookie.generator {
		ManifestSessionCookieGenerator::RandomBase36Concat { segments } => {
			let seed = pseudo_random_seed(manifest);
			let mut token = String::new();
			for (index, segment) in segments.iter().enumerate() {
				token.push_str(&pseudo_random_segment(&seed, index, segment));
			}
			token
		}
	}
}

pub(crate) fn current_decryption_passphrase_for_manifest(manifest: &ClientManifest) -> String {
	let date = current_utc_date_string();

	match &manifest.decrypt.passphrase {
		ManifestPassphraseStrategy::UtcMd5Derived {
			date_format: _,
			prefix,
			salt,
			suffix,
			digest_encoding: _,
			digest_slice,
		} => build_md5_passphrase(prefix, salt, suffix, digest_slice, &date),
		ManifestPassphraseStrategy::UtcSha256Derived {
			date_format: _,
			prefix,
			salt,
			suffix,
			digest_encoding: _,
			digest_slice,
		} => build_sha256_passphrase(prefix, salt, suffix, digest_slice, &date),
	}
}

fn fetch_remote_manifest() -> Option<ClientManifest> {
	source_log!("[toonlivre] fetch_remote_manifest url={REMOTE_MANIFEST_URL}");
	let response = Request::get(REMOTE_MANIFEST_URL)
		.ok()?
		.header("accept", "application/json")
		.header("accept-language", ACCEPT_LANGUAGE)
		.header("user-agent", FALLBACK_USER_AGENT)
		.send()
		.ok()?;
	let status = response.status_code();
	if !(200..300).contains(&status) {
		source_log!("[toonlivre] fetch_remote_manifest status={status}");
		return None;
	}
	let body = response.get_string().ok()?;
	source_log!(
		"[toonlivre] fetch_remote_manifest body={}",
		manifest_log_snippet(&body)
	);
	parse_manifest(&body)
}

pub(crate) fn parse_manifest(manifest_json: &str) -> Option<ClientManifest> {
	let manifest = serde_json::from_str::<ClientManifest>(manifest_json).ok()?;
	validate_manifest(manifest)
}

fn validate_manifest(manifest: ClientManifest) -> Option<ClientManifest> {
	if manifest.schema_version != 1 {
		return None;
	}
	if manifest.source_id != MANIFEST_SOURCE_ID {
		return None;
	}
	if manifest.site_url.trim() != BASE_URL {
		return None;
	}
	if manifest.request.user_agent.trim().is_empty()
		|| manifest.request.accept_language.trim().is_empty()
		|| manifest.request.signature_header.trim().is_empty()
		|| (manifest.request.signature_rules.is_empty()
			&& manifest.request.signature_strategy.is_none())
		|| manifest.request.session_cookie.name.trim().is_empty()
		|| manifest
			.request
			.session_cookie
			.mirrors_into
			.iter()
			.any(|header_name| header_name.trim().is_empty())
		|| manifest.decrypt.data_key_header.trim().is_empty()
		|| manifest.decrypt.payload_selector.trim().is_empty()
		|| manifest.decrypt.algorithm != "cryptojs-rabbit"
	{
		return None;
	}
	if let Some(verify_header) = manifest.request.verify_header.as_ref() {
		if verify_header.trim().is_empty() {
			return None;
		}
	}
	if let Some(strategy) = manifest.request.signature_strategy.as_ref() {
		match strategy {
			ManifestSignatureStrategy::TimeSha256Base64 {
				timestamp_divisor,
				salt,
				route_selector,
			} => {
				if *timestamp_divisor <= 0
					|| salt.trim().is_empty()
					|| route_selector.when_url_contains.trim().is_empty()
					|| route_selector.when_matched.trim().is_empty()
					|| route_selector.otherwise.trim().is_empty()
				{
					return None;
				}
			}
			ManifestSignatureStrategy::SeedJwt {
				meta_name,
				endpoint_path,
				token_field,
			} => {
				if meta_name.trim().is_empty()
					|| endpoint_path.trim().is_empty()
					|| token_field.trim().is_empty()
				{
					return None;
				}
			}
		}
	}
	match &manifest.request.session_cookie.generator {
		ManifestSessionCookieGenerator::RandomBase36Concat { segments } => {
			if segments.is_empty() || segments.iter().any(|segment| segment.end <= segment.start) {
				return None;
			}
		}
	}
	match &manifest.decrypt.passphrase {
		ManifestPassphraseStrategy::UtcMd5Derived {
			date_format,
			prefix,
			salt,
			suffix,
			digest_encoding,
			digest_slice,
		}
		| ManifestPassphraseStrategy::UtcSha256Derived {
			date_format,
			prefix,
			salt,
			suffix,
			digest_encoding,
			digest_slice,
		} => {
			if date_format != "YYYY-MM-DD"
				|| digest_encoding != "hex"
				|| prefix.trim().is_empty()
				|| salt.trim().is_empty()
				|| suffix.trim().is_empty()
				|| digest_slice.end <= digest_slice.start
			{
				return None;
			}
		}
	}
	Some(manifest)
}

fn default_manifest() -> ClientManifest {
	ClientManifest {
		schema_version: 1,
		source_id: String::from(MANIFEST_SOURCE_ID),
		site_url: String::from(BASE_URL),
		request: ManifestRequest {
			user_agent: String::from(FALLBACK_USER_AGENT),
			accept_language: String::from(ACCEPT_LANGUAGE),
			signature_header: String::from(FALLBACK_SIGNATURE_HEADER),
			signature_rules: vec![],
			signature_strategy: Some(ManifestSignatureStrategy::SeedJwt {
				meta_name: String::from(FALLBACK_SEED_META_NAME),
				endpoint_path: String::from(FALLBACK_SEED_ENDPOINT_PATH),
				token_field: String::from(FALLBACK_SEED_TOKEN_FIELD),
			}),
			verify_header: None,
			include_credentials: true,
			session_cookie: ManifestSessionCookie {
				name: String::from(FALLBACK_COOKIE_NAME),
				generator: ManifestSessionCookieGenerator::RandomBase36Concat {
					segments: vec![
						ManifestRandomSegment {
							radix: 36,
							start: 2,
							end: 15,
						},
						ManifestRandomSegment {
							radix: 36,
							start: 2,
							end: 15,
						},
					],
				},
				mirrors_into: vec![],
			},
		},
		decrypt: DecryptManifest {
			data_key_header: String::from(FALLBACK_DATA_KEY_HEADER),
			payload_selector: String::from("header-named-or-first-string"),
			algorithm: String::from("cryptojs-rabbit"),
			passphrase: ManifestPassphraseStrategy::UtcSha256Derived {
				date_format: String::from("YYYY-MM-DD"),
				prefix: String::from(FALLBACK_DECRYPTION_PREFIX),
				salt: String::from(FALLBACK_DECRYPTION_SALT),
				suffix: String::from(FALLBACK_DECRYPTION_SUFFIX),
				digest_encoding: String::from("hex"),
				digest_slice: ManifestDigestSlice { start: 0, end: 8 },
			},
		},
	}
}

fn manifest_capability_score(manifest: &ClientManifest) -> i32 {
	let signature_score = match manifest.request.signature_strategy.as_ref() {
		Some(ManifestSignatureStrategy::SeedJwt { .. }) => 30,
		Some(ManifestSignatureStrategy::TimeSha256Base64 { .. }) => 20,
		None => 10,
	};
	let passphrase_score = match manifest.decrypt.passphrase {
		ManifestPassphraseStrategy::UtcSha256Derived { .. } => 6,
		ManifestPassphraseStrategy::UtcMd5Derived { .. } => 3,
	};

	signature_score + passphrase_score
}

fn select_static_signature_value(rules: &[ManifestSignatureRule], url: &str) -> String {
	for rule in rules.iter() {
		if rule.default {
			continue;
		}
		if rule
			.when
			.as_ref()
			.map(|matcher| !matcher.url_contains.is_empty() && url.contains(&matcher.url_contains))
			.unwrap_or(false)
		{
			return rule.value.clone();
		}
	}

	rules
		.iter()
		.find(|rule| rule.default)
		.map(|rule| rule.value.clone())
		.unwrap_or_default()
}

fn dynamic_signature_value(
	manifest: &ClientManifest,
	strategy: &ManifestSignatureStrategy,
	url: &str,
	session_token: &str,
) -> String {
	match strategy {
		ManifestSignatureStrategy::TimeSha256Base64 {
			timestamp_divisor,
			salt,
			route_selector,
		} => {
			let timestamp = current_date()
				.saturating_mul(1_000)
				.div_euclid(*timestamp_divisor);
			let route_kind = if url.contains(&route_selector.when_url_contains) {
				route_selector.when_matched.as_str()
			} else {
				route_selector.otherwise.as_str()
			};
			let payload = format!("{timestamp}:{route_kind}:{salt}");
			let digest = sha256_hex(payload.as_bytes());
			STANDARD.encode(format!("{timestamp}:{digest}").as_bytes())
		}
		ManifestSignatureStrategy::SeedJwt {
			endpoint_path,
			token_field,
			..
		} => seed_jwt_signature_value(manifest, endpoint_path, token_field, session_token),
	}
}

fn seed_jwt_signature_value(
	manifest: &ClientManifest,
	endpoint_path: &str,
	token_field: &str,
	session_token: &str,
) -> String {
	let cached = cached_seed_token();
	if let Some(token) = cached.as_ref() {
		if seed_token_is_fresh(token) {
			return token.clone();
		}
	}

	if let Some(token) = fetch_seed_token(manifest, endpoint_path, token_field, session_token) {
		cache_seed_token(&token);
		return token;
	}

	cached.unwrap_or_default()
}

fn cached_seed_token() -> Option<String> {
	let token = defaults_get::<String>(MANIFEST_SEED_TOKEN_KEY)?;
	if token.trim().is_empty() {
		return None;
	}

	Some(token)
}

fn fetch_seed_token(
	manifest: &ClientManifest,
	endpoint_path: &str,
	token_field: &str,
	session_token: &str,
) -> Option<String> {
	let seed_url = site_url_with_path(&manifest.site_url, endpoint_path);
	let cookie = format!("{}={session_token}", manifest.request.session_cookie.name);
	let response = Request::get(seed_url.as_str())
		.ok()?
		.header("accept", "application/json")
		.header("accept-language", &manifest.request.accept_language)
		.header("user-agent", &manifest.request.user_agent)
		.header("origin", &manifest.site_url)
		.header("referer", &manifest.site_url)
		.header("cookie", &cookie)
		.send()
		.ok()?;
	if !(200..300).contains(&response.status_code()) {
		return None;
	}

	let body = response.get_string().ok()?;
	let parsed = serde_json::from_str::<serde_json::Value>(&body).ok()?;
	let token = parsed.get(token_field)?.as_str()?.trim();
	if token.is_empty() {
		return None;
	}

	Some(String::from(token))
}

fn cache_seed_token(token: &str) {
	defaults_set(
		MANIFEST_SEED_TOKEN_KEY,
		DefaultValue::String(String::from(token)),
	);
}

fn seed_token_is_fresh(token: &str) -> bool {
	decode_jwt_expiry_millis(token)
		.map(|expiry_millis| {
			expiry_millis > current_date().saturating_mul(1_000) + SEED_TOKEN_EXPIRY_MARGIN_MS
		})
		.unwrap_or(false)
}

fn decode_jwt_expiry_millis(token: &str) -> Option<i64> {
	let segments: Vec<&str> = token.split('.').collect();
	if segments.len() != 3 {
		return None;
	}

	let mut normalized = String::from(segments[1]);
	normalized = normalized.replace('-', "+");
	normalized = normalized.replace('_', "/");
	while normalized.len() % 4 != 0 {
		normalized.push('=');
	}

	let payload = STANDARD.decode(normalized.as_bytes()).ok()?;
	let parsed = serde_json::from_slice::<serde_json::Value>(&payload).ok()?;
	parsed
		.get("exp")?
		.as_i64()
		.map(|expiry| expiry.saturating_mul(1_000))
}

fn site_url_with_path(site_url: &str, path: &str) -> String {
	if path.starts_with("http://") || path.starts_with("https://") {
		return String::from(path);
	}

	if path.starts_with('/') {
		return format!("{}{}", site_url.trim_end_matches('/'), path);
	}

	format!("{}/{}", site_url.trim_end_matches('/'), path)
}

fn build_md5_passphrase(
	prefix: &str,
	salt: &str,
	suffix: &str,
	digest_slice: &ManifestDigestSlice,
	date: &str,
) -> String {
	let seed = format!("{date}{salt}{suffix}");
	let mut hasher = Md5::new();
	hasher.update(seed.as_bytes());
	let digest = hasher.finalize();
	let digest_hex = hex_string(&digest);
	let start = digest_slice.start.min(digest_hex.len());
	let end = digest_slice.end.min(digest_hex.len()).max(start);
	format!("{prefix}{}", &digest_hex[start..end])
}

fn build_sha256_passphrase(
	prefix: &str,
	salt: &str,
	suffix: &str,
	digest_slice: &ManifestDigestSlice,
	date: &str,
) -> String {
	let seed = format!("{date}{salt}{suffix}");
	let digest_hex = sha256_hex(seed.as_bytes());
	let start = digest_slice.start.min(digest_hex.len());
	let end = digest_slice.end.min(digest_hex.len()).max(start);
	format!("{prefix}{}", &digest_hex[start..end])
}

fn sha256_hex(bytes: &[u8]) -> String {
	let digest = <Sha256 as sha2::Digest>::digest(bytes);
	hex_string(digest.as_slice())
}

fn pseudo_random_seed(manifest: &ClientManifest) -> String {
	let counter = next_request_counter();
	format!(
		"{}:{counter}:{}:{}",
		current_date(),
		manifest.source_id,
		current_decryption_passphrase_for_manifest(manifest)
	)
}

fn next_request_counter() -> i32 {
	let next = defaults_get::<i32>(MANIFEST_REQUEST_COUNTER_KEY)
		.unwrap_or_default()
		.wrapping_add(1);
	defaults_set(MANIFEST_REQUEST_COUNTER_KEY, DefaultValue::Int(next));
	next
}

fn pseudo_random_segment(seed: &str, index: usize, segment: &ManifestRandomSegment) -> String {
	let target_len = segment.end.max(segment.start + 1);
	let mut stream = String::new();
	let mut round = 0usize;
	while stream.len() < target_len {
		let mut hasher = Md5::new();
		hasher.update(format!("{seed}:{index}:{round}").as_bytes());
		let digest = hasher.finalize();
		for byte in digest.iter() {
			stream.push(base36_digit(byte % segment.radix.max(2)));
			if stream.len() >= target_len {
				break;
			}
			stream.push(base36_digit(
				(byte / segment.radix.max(2)) % segment.radix.max(2),
			));
			if stream.len() >= target_len {
				break;
			}
		}
		round += 1;
	}
	String::from(&stream[segment.start..target_len])
}

fn base36_digit(value: u8) -> char {
	match value {
		0..=9 => (b'0' + value) as char,
		_ => (b'a' + (value - 10)) as char,
	}
}

fn hex_string(bytes: &[u8]) -> String {
	let mut output = String::new();
	for byte in bytes.iter() {
		output.push(hex_digit(byte >> 4));
		output.push(hex_digit(byte & 0x0F));
	}
	output
}

fn hex_digit(value: u8) -> char {
	match value {
		0..=9 => (b'0' + value) as char,
		_ => (b'a' + (value - 10)) as char,
	}
}

fn manifest_log_snippet(body: &str) -> String {
	let trimmed = body.trim();
	if trimmed.is_empty() {
		return String::new();
	}
	let mut output = String::new();
	let mut previous_was_space = false;
	for ch in trimmed.chars() {
		let normalized = if ch.is_whitespace() { ' ' } else { ch };
		if normalized == ' ' {
			if previous_was_space {
				continue;
			}
			previous_was_space = true;
		} else {
			previous_was_space = false;
		}
		if output.len() >= 400 {
			output.push_str("...");
			break;
		}
		output.push(normalized);
	}
	output
}

fn current_utc_date_string() -> String {
	let days = current_date().div_euclid(86_400);
	let (year, month, day) = civil_from_days(days);
	format!("{year:04}-{month:02}-{day:02}")
}

fn civil_from_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
	let z = days_since_unix_epoch + 719_468;
	let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
	let doe = z - era * 146_097;
	let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
	let y = yoe + era * 400;
	let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
	let mp = (5 * doy + 2) / 153;
	let day = doy - (153 * mp + 2) / 5 + 1;
	let month = mp + if mp < 10 { 3 } else { -9 };
	let year = y + if month <= 2 { 1 } else { 0 };
	(year as i32, month as u32, day as u32)
}
