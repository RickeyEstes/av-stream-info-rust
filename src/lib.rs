//! This library can analyze a http/https address and check if leads to an audio or a video stream
//! If so, then it will extract information about the stream from its metadata or in case of HLS streams
//! from its master playlist file.

extern crate native_tls;
extern crate playlist_decoder;
extern crate url;
extern crate hls_m3u8;
extern crate log;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate tree_magic;

mod request;
mod streamcheck;
mod streamdeepscan;

use std::time::Duration;
use std::thread;

/// Check url for audio/video stream.
/// # Example
/// ```rust
/// let list = av_stream_info_rust::check("https://example.com/test.m3u", 10, 3, 3);
/// for item in list {
///     println!("{:?}", item);
/// }
/// ```
/// # Arguments
/// * `url` - The url to check
/// * `timeout` - TCP timeout for connect and read in seconds
/// * `max_depth` - How many layers of http redirects or playlists should be followed
/// * `retries` - Retry how many times to find at least one working stream
pub fn check(
    url: &str,
    timeout: u32,
    max_depth: u8,
    retries: u8,
) -> Vec<streamcheck::StreamCheckResult> {
    let mut working = false;
    let mut list: Vec<streamcheck::StreamCheckResult> = Vec::new();

    for _i in 0..retries {
        list = streamcheck::check(url, false, timeout, max_depth);
        for item in list.iter() {
            match item {
                &Ok(_) => {
                    working = true;
                    break;
                }
                &Err(_) => {}
            }
        }

        if working {
            break;
        }

        thread::sleep(Duration::from_secs(1));
    }
    list
}