//! Background image fetcher with in-memory cache.
//!
//! Fetches album art from Sonos speakers over HTTP in a background thread,
//! decodes images via the `image` crate, and caches decoded `DynamicImage`s
//! by URL. The main thread polls for completed loads each event loop tick.
//!
//! Design: single-threaded access from the TUI event loop. `request()` uses
//! `RefCell` for the pending set so it can be called from render functions
//! that only have `&self` access.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc;
use std::time::Duration;

use image::imageops::FilterType;
use image::DynamicImage;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

/// Maximum number of cached images before evicting oldest entries.
const MAX_CACHE_SIZE: usize = 20;

/// HTTP fetch timeout for album art requests.
const FETCH_TIMEOUT: Duration = Duration::from_secs(3);

struct LoadRequest {
    url: String,
}

struct LoadResult {
    url: String,
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

impl Default for ImageLoader {
    fn default() -> Self {
        Self::new()
    }
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
    pub fn request(&self, url: &str) {
        if self.cache.contains_key(url) {
            return;
        }

        let mut pending = self.pending.borrow_mut();
        if pending.contains(url) {
            return;
        }

        let req = LoadRequest {
            url: url.to_string(),
        };

        if self.request_tx.send(req).is_ok() {
            pending.insert(url.to_string());
        }
    }

    /// Drain completed fetches into the cache. Call from event loop each tick.
    ///
    /// Returns `true` if any new images were loaded (should mark app dirty).
    pub fn poll(&mut self) -> bool {
        let mut loaded = false;
        while let Ok(result) = self.result_rx.try_recv() {
            self.pending.borrow_mut().remove(&result.url);
            if let Some(img) = result.image {
                self.evict_if_full();
                self.insertion_order.push_back(result.url.clone());
                self.cache.insert(result.url, img);
                loaded = true;
            }
        }
        loaded
    }

    pub fn get(&self, url: &str) -> Option<&DynamicImage> {
        self.cache.get(url)
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

/// Worker thread: receives fetch requests, downloads + decodes images, sends results back.
fn worker_loop(rx: mpsc::Receiver<LoadRequest>, tx: mpsc::Sender<LoadResult>) {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(FETCH_TIMEOUT))
        .build()
        .new_agent();

    for req in rx {
        let image = fetch_and_decode(&agent, &req.url);
        let result = LoadResult {
            url: req.url,
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

    let img = image::load_from_memory(&body)
        .map_err(|e| {
            tracing::debug!("Album art decode failed for {url}: {e}");
            e
        })
        .ok()?;

    Some(crop_to_square(img))
}

/// Center-crop an image to a square using the shorter dimension.
fn crop_to_square(img: DynamicImage) -> DynamicImage {
    let (w, h) = (img.width(), img.height());
    if w == h {
        return img;
    }
    let side = w.min(h);
    let x = (w - side) / 2;
    let y = (h - side) / 2;
    img.crop_imm(x, y, side, side)
}

// ---------------------------------------------------------------------------
// Render protocol state
// ---------------------------------------------------------------------------

/// Album art size in terminal cells.
const ART_COLS: u16 = 6;
const ART_ROWS: u16 = 3;

/// Hook-friendly state for album art protocol lifecycle.
///
/// Tracks the current album art URI and holds the `StatefulProtocol` used for
/// rendering. Detects URI changes, invalidates stale protocols, and lazily
/// creates new ones from the image cache.
#[derive(Default)]
pub struct ArtProtocolState {
    url: Option<String>,
    pub protocol: Option<StatefulProtocol>,
}

impl ArtProtocolState {
    /// Update protocol when URI changes. Creates protocol from cached image.
    ///
    /// Call this each render frame with the current `art_url`. Handles:
    /// - URI change detection (invalidates old protocol)
    /// - Lazy protocol creation from `ImageLoader` cache + `Picker`
    pub fn ensure_protocol(
        &mut self,
        art_url: &Option<String>,
        image_loader: &ImageLoader,
        picker: &RefCell<Option<Picker>>,
    ) {
        let url_changed = self.url.as_deref() != art_url.as_deref();
        if url_changed {
            self.url = art_url.clone();
            self.protocol = None;
        }

        if self.protocol.is_none() {
            if let Some(ref url) = art_url {
                if let Some(img) = image_loader.get(url) {
                    if let Some(ref mut p) = *picker.borrow_mut() {
                        let (fw, fh) = p.font_size();
                        let target_w = (ART_COLS * fw) as u32;
                        let target_h = (ART_ROWS * fh) as u32;
                        let resized = img.resize_exact(target_w, target_h, FilterType::Lanczos3);
                        self.protocol = Some(p.new_resize_protocol(resized));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_to_square_landscape() {
        let img = DynamicImage::new_rgb8(160, 90);
        let cropped = crop_to_square(img);
        assert_eq!(cropped.width(), 90);
        assert_eq!(cropped.height(), 90);
    }

    #[test]
    fn crop_to_square_portrait() {
        let img = DynamicImage::new_rgb8(90, 160);
        let cropped = crop_to_square(img);
        assert_eq!(cropped.width(), 90);
        assert_eq!(cropped.height(), 90);
    }

    #[test]
    fn crop_to_square_already_square() {
        let img = DynamicImage::new_rgb8(100, 100);
        let cropped = crop_to_square(img);
        assert_eq!(cropped.width(), 100);
        assert_eq!(cropped.height(), 100);
    }
}
