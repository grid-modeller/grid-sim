//! HTTP fetching with retry and exponential backoff.
//!
//! The only network access in the whole tool (docs/05: the pack is
//! fetched and built locally, never shipped). Fixed URLs, no
//! authentication, blocking requests via `ureq`. Overnight-friendly:
//! transient failures (transport errors, HTTP 408/429/5xx) are retried
//! with exponential backoff; anything else fails fast. Existing raw
//! files are never re-fetched (resume semantics live in the caller).

use std::path::Path;
use std::time::Duration;

use super::error::FetchDataError;

/// Retry attempts per URL (first try + 4 retries).
const ATTEMPTS: u32 = 5;
/// Backoff before retry attempt `n` (attempts are numbered from 2) is
/// `BASE_BACKOFF_SECS << (n - 1)`: 8 s, 16 s, 32 s, 64 s.
const BASE_BACKOFF_SECS: u64 = 4;
/// Per-request timeout. The largest source (a monthly FUELHH chunk,
/// ~6 MB) downloads in seconds; five minutes matches the Python scripts.
const TIMEOUT_SECS: u64 = 300;
/// Response-size ceiling: comfortably above the ~6 MB monthly chunks,
/// far below anything pathological.
const MAX_RESPONSE_BYTES: u64 = 256 * 1024 * 1024;

/// GET `url` and return the response body.
///
/// Retries transient failures with exponential backoff; returns
/// [`FetchDataError::Fetch`] carrying the final error once retries are
/// exhausted, or immediately for non-transient failures.
pub fn get(url: &str) -> Result<Vec<u8>, FetchDataError> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(TIMEOUT_SECS)))
        .build()
        .into();

    let mut last_error = String::new();
    for attempt in 1..=ATTEMPTS {
        if attempt > 1 {
            let backoff = BASE_BACKOFF_SECS << (attempt - 1);
            eprintln!("  retry {attempt}/{ATTEMPTS} for {url} in {backoff}s ({last_error})");
            std::thread::sleep(Duration::from_secs(backoff));
        }
        match agent.get(url).call() {
            Ok(mut response) => {
                return response
                    .body_mut()
                    .with_config()
                    .limit(MAX_RESPONSE_BYTES)
                    .read_to_vec()
                    .map_err(|e| FetchDataError::Fetch {
                        url: url.to_owned(),
                        attempts: attempt,
                        reason: format!("reading response body: {e}"),
                    });
            }
            Err(ureq::Error::StatusCode(code)) if is_transient_status(code) => {
                last_error = format!("HTTP {code}");
            }
            Err(ureq::Error::StatusCode(code)) => {
                return Err(FetchDataError::Fetch {
                    url: url.to_owned(),
                    attempts: attempt,
                    reason: format!("HTTP {code} (not retryable)"),
                });
            }
            Err(transport) => {
                last_error = transport.to_string();
            }
        }
    }
    Err(FetchDataError::Fetch {
        url: url.to_owned(),
        attempts: ATTEMPTS,
        reason: last_error,
    })
}

/// GET `url` into `path`, unless `path` already exists (resume semantics:
/// a previously fetched raw file is authoritative — Elexon revises
/// published data, and re-fetching would silently change the pack).
/// Returns whether a fetch happened.
pub fn get_to_file(url: &str, path: &Path) -> Result<bool, FetchDataError> {
    if path.exists() {
        println!("skip (exists): {}", path.display());
        return Ok(false);
    }
    println!("fetching {url}");
    let body = get(url)?;
    // Write via a temp name so an interrupted run never leaves a partial
    // file that a resume would then trust.
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, &body).map_err(|source| FetchDataError::io(&tmp, source))?;
    std::fs::rename(&tmp, path).map_err(|source| FetchDataError::io(path, source))?;
    println!("  {} bytes -> {}", body.len(), path.display());
    Ok(true)
}

/// Transient HTTP statuses worth a retry: timeout, throttling, server
/// errors.
fn is_transient_status(code: u16) -> bool {
    code == 408 || code == 429 || (500..=599).contains(&code)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn transient_statuses_are_the_retryable_set() {
        for code in [408, 429, 500, 502, 503, 504, 599] {
            assert!(is_transient_status(code), "{code} should be transient");
        }
        for code in [200, 301, 400, 401, 403, 404, 410] {
            assert!(!is_transient_status(code), "{code} should not be transient");
        }
    }
}
