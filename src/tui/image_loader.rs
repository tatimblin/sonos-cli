//! Background image fetcher with in-memory cache.
//!
//! Fetches album art from Sonos speakers over HTTP in a background thread,
//! decodes images via the `image` crate, and caches decoded `DynamicImage`s
//! by URI. The main thread polls for completed loads each event loop tick.
//!
//! Design: single-threaded access from the TUI event loop. `request()` uses
//! `RefCell` for the pending set so it can be called from render functions
//! that only have `&self` access.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::net::IpAddr;
use std::sync::mpsc;
use std::time::Duration;

use image::DynamicImage;

/// Maximum number of cached images before evicting oldest entries.
const MAX_CACHE_SIZE: usize = 20;

/// HTTP fetch timeout for album art requests.
const FETCH_TIMEOUT: Duration = Duration::from_secs(3);

struct LoadRequest {
    uri: String,
    full_url: String,
}

struct LoadResult {
    uri: String,
    image: Option<DynamicImage>,
}

/// Background image loader with request/poll/get API.
///
/// - `request()` — enqueue a fetch (callable from render with `&self`)
/// - `poll()` — drain completed fetches into cache (callable from event loop with `&mut self`)
/// - `get()` — read from cache (callable from render with `&self`)
pub struct ImageLoader {
    cache: HashMap<String, DynamicImage>,
    /// Insertion order for LRU eviction.
    insertion_order: VecDeque<String>,
    /// URIs currently being fetched (RefCell for &self access from render).
    pending: RefCell<HashSet<String>>,
    result_rx: mpsc::Receiver<LoadResult>,
    request_tx: mpsc::Sender<LoadRequest>,
}

impl ImageLoader {
    pub fn new() -> Self {
        let (request_tx, request_rx) = mpsc::channel::<LoadRequest>();
        let (result_tx, result_rx) = mpsc::channel::<LoadResult>();

        // Spawn a single worker thread for fetching images
        std::thread::Builder::new()
            .name("album-art-loader".into())
            .spawn(move || {
                worker_loop(request_rx, result_tx);
            })
            .expect("failed to spawn album art loader thread");

        Self {
            cache: HashMap::new(),
            insertion_order: VecDeque::new(),
            pending: RefCell::new(HashSet::new()),
            result_rx,
            request_tx,
        }
    }

    /// Request an image fetch if not already cached or pending.
    ///
    /// Callable from render functions with `&self`. The fetch happens in a
    /// background thread; call `poll()` from the event loop to collect results.
    pub fn request(&self, uri: &str, speaker_ip: IpAddr) {
        if self.cache.contains_key(uri) {
            return;
        }

        let mut pending = self.pending.borrow_mut();
        if pending.contains(uri) {
            return;
        }

        let full_url = build_url(uri, speaker_ip);
        let req = LoadRequest {
            uri: uri.to_string(),
            full_url,
        };

        if self.request_tx.send(req).is_ok() {
            pending.insert(uri.to_string());
        }
    }

    /// Drain completed fetches into the cache. Call from event loop each tick.
    ///
    /// Returns `true` if any new images were loaded (should mark app dirty).
    pub fn poll(&mut self) -> bool {
        let mut loaded = false;
        while let Ok(result) = self.result_rx.try_recv() {
            self.pending.borrow_mut().remove(&result.uri);
            if let Some(img) = result.image {
                self.evict_if_full();
                self.insertion_order.push_back(result.uri.clone());
                self.cache.insert(result.uri, img);
                loaded = true;
            }
        }
        loaded
    }

    /// Get a cached image by URI.
    pub fn get(&self, uri: &str) -> Option<&DynamicImage> {
        self.cache.get(uri)
    }

    fn evict_if_full(&mut self) {
        while self.cache.len() >= MAX_CACHE_SIZE {
            if let Some(oldest) = self.insertion_order.pop_front() {
                self.cache.remove(&oldest);
            } else {
                break;
            }
        }
    }
}

/// Build a full URL from an album art URI and speaker IP.
///
/// Sonos speakers return `album_art_uri` as either:
/// - A relative path: `/getaa?s=1&u=...` → prepend `http://{ip}:1400`
/// - An absolute URL: `http://...` → use as-is
fn build_url(uri: &str, speaker_ip: IpAddr) -> String {
    if uri.starts_with("http://") || uri.starts_with("https://") {
        uri.to_string()
    } else {
        format!("http://{}:1400{}", speaker_ip, uri)
    }
}

/// Worker thread: receives fetch requests, downloads + decodes images, sends results back.
fn worker_loop(rx: mpsc::Receiver<LoadRequest>, tx: mpsc::Sender<LoadResult>) {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(FETCH_TIMEOUT))
        .build()
        .new_agent();

    for req in rx {
        let image = fetch_and_decode(&agent, &req.full_url);
        let result = LoadResult {
            uri: req.uri,
            image,
        };
        if tx.send(result).is_err() {
            break; // main thread dropped, exit
        }
    }
}

/// Fetch an image over HTTP and decode it.
fn fetch_and_decode(agent: &ureq::Agent, url: &str) -> Option<DynamicImage> {
    let response = agent.get(url).call().ok()?;

    let status = response.status();
    if status != 200 {
        tracing::debug!("Album art fetch returned {status} for {url}");
        return None;
    }

    // Read body into memory (limit to 5MB to prevent OOM)
    let body = response
        .into_body()
        .with_config()
        .limit(5 * 1024 * 1024)
        .read_to_vec()
        .ok()?;

    image::load_from_memory(&body)
        .map_err(|e| {
            tracing::debug!("Album art decode failed for {url}: {e}");
            e
        })
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_url_relative_path() {
        let ip: IpAddr = "192.168.1.100".parse().unwrap();
        assert_eq!(
            build_url("/getaa?s=1&u=test", ip),
            "http://192.168.1.100:1400/getaa?s=1&u=test"
        );
    }

    #[test]
    fn build_url_absolute_http() {
        let ip: IpAddr = "192.168.1.100".parse().unwrap();
        let url = "http://example.com/art.jpg";
        assert_eq!(build_url(url, ip), url);
    }

    #[test]
    fn build_url_absolute_https() {
        let ip: IpAddr = "192.168.1.100".parse().unwrap();
        let url = "https://example.com/art.jpg";
        assert_eq!(build_url(url, ip), url);
    }
}
