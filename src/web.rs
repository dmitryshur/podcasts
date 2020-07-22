use crate::Errors;
use bytes::Bytes;
#[cfg(not(test))]
use rayon::prelude::*;
#[cfg(not(test))]
use reqwest;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::StatusCode;
use std::io::{self, Read, Write};

pub struct Web {
    client: reqwest::blocking::Client,
}

struct DownloadBuffer {
    inner: Vec<u8>,
    bytes_count: u64,
    // Content-Length header might be missing
    progress_bar: ProgressBar,
}

impl DownloadBuffer {
    fn new(total_size: Option<u64>, progress_bars: &MultiProgress) -> Self {
        // TODO show a spinner if total_size = None
        let progress_bar = progress_bars.add(ProgressBar::new(total_size.unwrap()));
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .progress_chars("#>-"),
        );

        Self {
            inner: vec![],
            bytes_count: 0,
            progress_bar,
        }
    }
}

impl Write for DownloadBuffer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.bytes_count += (written as u64);
        self.progress_bar.set_position(self.bytes_count);

        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Web {
    pub fn new(timeout: std::time::Duration) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(if timeout == std::time::Duration::from_secs(0) {
                None
            } else {
                Some(timeout)
            })
            .build()
            .expect("Can't create reqwest client");
        Self { client }
    }

    #[cfg(not(test))]
    pub fn get<'a>(&self, urls: &[&'a str]) -> Vec<(&'a str, Result<Bytes, Errors>)> {
        let progress_bars = MultiProgress::new();

        let responses: Vec<(&str, Result<Bytes, Errors>)> = urls
            .par_iter()
            .map(|url| {
                println!("Fetching URL {}", *url);

                let bytes = self.client.get(*url).send();
                return match bytes {
                    Ok(mut response) => {
                        if response.status() == StatusCode::NOT_FOUND {
                            return (*url, Err(Errors::NotFound((*url).to_string())));
                        }

                        let content_length = response.content_length();
                        let mut buffer = DownloadBuffer::new(content_length, &progress_bars);
                        let bytes_count = response.copy_to(&mut buffer);

                        if let Ok(count) = bytes_count {
                            return (*url, Ok(Bytes::copy_from_slice(&buffer.inner)));
                        }

                        (*url, Err(Errors::Network(bytes_count.err().unwrap())))
                    }
                    Err(error) => {
                        if error.is_timeout() {
                            return (*url, Err(Errors::Timeout((*url).to_string())));
                        }

                        (*url, Err(Errors::Network(error)))
                    }
                };
            })
            .collect();

        progress_bars.join_and_clear().unwrap();

        responses
    }

    #[cfg(test)]
    pub fn get<'a>(&self, urls: &[&'a str]) -> Vec<(&'a str, Result<Bytes, ()>)> {
        // The tests work with two files - rss_203.xml, syntax.xml, which contain valid RSS data
        let responses: Vec<(&str, Result<Bytes, ()>)> = urls
            .iter()
            .map(|url| {
                let bytes = if *url == "http://feeds.feedburner.com/Http203Podcast" {
                    let mut rss_203 = std::fs::File::open("src/rss_203.xml").expect("Can't open rss_203 file");
                    let mut rss_203_contents = String::new();
                    rss_203
                        .read_to_string(&mut rss_203_contents)
                        .expect("Can't get rss_203 contents");
                    Ok(Bytes::from(rss_203_contents))
                } else {
                    let mut syntax = std::fs::File::open("src/syntax.xml").expect("Can't open syntax file");
                    let mut syntax_contents = String::new();
                    syntax
                        .read_to_string(&mut syntax_contents)
                        .expect("Can't get syntax contents");
                    Ok(Bytes::from(syntax_contents))
                };

                (*url, bytes)
            })
            .collect();

        responses
    }
}
