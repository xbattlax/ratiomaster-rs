/// Trait for abstracting HTTP tracker communication.
///
/// Enables mocking tracker requests in tests without hitting real trackers.
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::network::http::{HttpError, HttpResponse, HttpVersion};
use crate::proxy::ProxyConfig;

/// Abstraction over HTTP tracker communication.
///
/// Both `announce` and `scrape` perform HTTP GET requests to tracker endpoints.
/// The default implementation [`HttpTrackerClient`] uses the built-in raw HTTP client.
pub trait TrackerClient: Send + Sync {
    /// Sends an HTTP GET request for a tracker announce.
    fn announce<'a>(
        &'a self,
        url: &'a str,
        headers: &'a [(String, String)],
        http_version: HttpVersion,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpError>> + Send + 'a>>;

    /// Sends an HTTP GET request for a tracker scrape.
    fn scrape<'a>(
        &'a self,
        url: &'a str,
        headers: &'a [(String, String)],
        http_version: HttpVersion,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpError>> + Send + 'a>>;
}

/// Default tracker client using the built-in raw HTTP client.
pub struct HttpTrackerClient {
    proxy: ProxyConfig,
    timeout: Duration,
}

impl HttpTrackerClient {
    /// Creates a new HTTP tracker client with the given proxy and timeout settings.
    pub fn new(proxy: ProxyConfig, timeout: Duration) -> Self {
        Self { proxy, timeout }
    }
}

impl TrackerClient for HttpTrackerClient {
    fn announce<'a>(
        &'a self,
        url: &'a str,
        headers: &'a [(String, String)],
        http_version: HttpVersion,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpError>> + Send + 'a>> {
        Box::pin(crate::network::http::get(
            url,
            headers,
            http_version,
            &self.proxy,
            self.timeout,
        ))
    }

    fn scrape<'a>(
        &'a self,
        url: &'a str,
        headers: &'a [(String, String)],
        http_version: HttpVersion,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, HttpError>> + Send + 'a>> {
        Box::pin(crate::network::http::get(
            url,
            headers,
            http_version,
            &self.proxy,
            self.timeout,
        ))
    }
}
