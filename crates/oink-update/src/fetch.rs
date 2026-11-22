//! Catalog lookup across the configured sources and HTTPS transfers, shared by
//! the Windows service and the macOS backend.
use crate::Error;

/// Tries each source in order and returns the first catalog that verifies.
/// Any failure moves on to the next source. When all fail, a verification
/// failure is reported in preference to a network failure: it says more.
pub fn first_verified<T>(
    sources: &[String],
    path: &str,
    mut get: impl FnMut(&str) -> Result<Vec<u8>, Error>,
    mut verify: impl FnMut(&[u8]) -> Result<T, Error>,
) -> Result<T, Error> {
    let mut failure = None;
    for source in sources {
        match get(&format!("{source}/{path}")).and_then(|bytes| verify(&bytes)) {
            Ok(value) => return Ok(value),
            Err(error) => {
                if failure.is_none_or(|first| first == Error::Network) {
                    failure = Some(error);
                }
            }
        }
    }
    Err(failure.unwrap_or(Error::NotConfigured))
}

/// GET with no redirects for catalogs; packages may follow HTTPS redirects to
/// their CDN. `progress` receives (received, expected) as bytes arrive.
#[cfg(feature = "http")]
pub fn get(url: &str, limit: u64, package: bool, progress: &mut dyn FnMut(u64, u64)) -> Result<Vec<u8>, Error> {
    get_until(url, limit, package, &mut |received, expected| {
        progress(received, expected);
        true
    })
}
/// As [`get`], but the transfer is abandoned (as `Error::Network`) as soon as
/// `progress` returns false.
#[cfg(all(feature = "http", not(feature = "test-feed")))]
pub fn get_until(_: &str, _: u64, _: bool, _: &mut dyn FnMut(u64, u64) -> bool) -> Result<Vec<u8>, Error> {
    Err(Error::NotConfigured)
}

#[cfg(all(feature = "http", feature = "test-feed"))]
pub fn get_until(
    url: &str,
    limit: u64,
    _package: bool,
    progress: &mut dyn FnMut(u64, u64) -> bool,
) -> Result<Vec<u8>, Error> {
    use std::{io::Read, time::Duration};
    let origin = url::Url::parse(crate::feed::feed_origin()).map_err(|_| Error::Network)?;
    let target = url::Url::parse(url).map_err(|_| Error::Network)?;
    if target.origin() != origin.origin() || !target.username().is_empty() || target.password().is_some() {
        return Err(Error::Network);
    }
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .user_agent("Oink-updater/2")
        .build()
        .map_err(|_| Error::Network)?;
    let response = client.get(target).send().map_err(|_| Error::Network)?;
    if response.status() != reqwest::StatusCode::OK || response.content_length().is_some_and(|n| n > limit) {
        return Err(Error::Network);
    }
    let expected = response.content_length().unwrap_or(limit);
    let mut body = response.take(limit + 1);
    let mut bytes = Vec::with_capacity(expected.min(limit) as usize);
    let mut chunk = vec![0; 256 * 1024];
    loop {
        let read = body.read(&mut chunk).map_err(|_| Error::Network)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.len() as u64 > limit {
            return Err(Error::Limit);
        }
        if !progress(bytes.len() as u64, expected) {
            return Err(Error::Network);
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(feature = "http", not(feature = "test-feed")))]
    #[test]
    fn production_transport_rejects_even_a_loopback_url_before_progress() {
        let result = get_until("http://127.0.0.1:9", 1024, false, &mut |_, _| {
            panic!("disabled transport must not report progress")
        });
        assert_eq!(result, Err(Error::NotConfigured));
    }

    fn sources() -> Vec<String> {
        vec![
            "https://a.example".into(),
            "https://b.example".into(),
            "https://c.example".into(),
        ]
    }
    #[test]
    fn the_first_source_that_verifies_wins() {
        let mut asked = vec![];
        let result = first_verified(
            &sources(),
            "windows/v2/x/manifest.json",
            |url| {
                asked.push(url.to_owned());
                match url {
                    "https://a.example/windows/v2/x/manifest.json" => Err(Error::Network),
                    "https://b.example/windows/v2/x/manifest.json" => Ok(b"stale".to_vec()),
                    _ => Ok(b"good".to_vec()),
                }
            },
            |bytes| if bytes == b"good" { Ok(7) } else { Err(Error::Replay) },
        );
        assert_eq!(result, Ok(7));
        assert_eq!(asked.len(), 3);
    }
    #[test]
    fn a_verification_failure_outranks_network_failures() {
        let fail = |error| {
            first_verified(
                &sources(),
                "p",
                |url| {
                    if url.starts_with("https://b") {
                        Ok(vec![])
                    } else {
                        Err(Error::Network)
                    }
                },
                |_| Err::<(), _>(error),
            )
        };
        assert_eq!(fail(Error::Signature), Err(Error::Signature));
        assert_eq!(
            first_verified(&sources(), "p", |_| Err(Error::Network), |_| Ok(())),
            Err(Error::Network)
        );
        assert_eq!(
            first_verified(&[], "p", |_| Ok(vec![]), |_| Ok(())),
            Err(Error::NotConfigured)
        );
    }
}
