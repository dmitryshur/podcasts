use crate::Errors;
use bytes::Bytes;
#[cfg(not(test))]
use rayon::prelude::*;
#[cfg(not(test))]
use reqwest;
use std::sync::Arc;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::StatusCode;
use std::io::{self, Write};

pub struct Web {
    client: reqwest::blocking::Client,
}

struct DownloadBuffer {
    inner: Vec<u8>,
    bytes_count: u64,
    progress_bar: ProgressBar,
}

impl DownloadBuffer {
    fn new(progress_bar: ProgressBar) -> Self {
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
        self.bytes_count += written as u64;
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
        let pbs = Arc::new(MultiProgress::new());
        let pbs_clone = Arc::clone(&pbs);

        // Used as a hack so that pbs won't finish right away
        let temp_pb = pbs.add(ProgressBar::hidden());
        let thread = std::thread::spawn(move || {
            let result = pbs_clone.join_and_clear();
            if let Err(_error) = result {
                println!("Progress bars error");
            }
        });

        let responses: Vec<(&str, Result<Bytes, Errors>)> = urls
            .par_iter()
            .map(|url| {
                let bytes = self.client.get(*url).send();
                return match bytes {
                    Ok(mut response) => {
                        if response.status() == StatusCode::NOT_FOUND {
                            return (*url, Err(Errors::NotFound((*url).to_string())));
                        }
                        let content_length = response.content_length();
                        let file_name: Vec<&str> = url.split('/').collect();
                        let file_name = file_name[file_name.len() - 1];

                        let pb_style = ProgressStyle::default_bar()
                            .template("{prefix} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                            .progress_chars("#>-");

                        let spinner_style = ProgressStyle::default_spinner()
                            .tick_strings(&["▹▹▹▹▹", "▸▹▹▹▹", "▹▸▹▹▹", "▹▹▸▹▹", "▹▹▹▸▹", "▹▹▹▹▸", "▪▪▪▪▪"])
                            .template("{spinner:.blue} {msg}");

                        // If Content-Length header was absent, draw a spinner. otherwise, draw a normal
                        // progress bar
                        let pb = if content_length.is_none() {
                            let spinner = pbs.add(ProgressBar::new_spinner());
                            spinner.set_style(spinner_style);
                            spinner.enable_steady_tick(120);
                            spinner.set_message(file_name);
                            spinner
                        } else {
                            let bar = pbs.add(ProgressBar::new(content_length.unwrap()));
                            bar.set_style(pb_style);
                            bar.set_prefix(file_name);
                            bar
                        };

                        let mut buffer = DownloadBuffer::new(pb);
                        let bytes_count = response.copy_to(&mut buffer);
                        temp_pb.finish_and_clear();

                        if let Ok(_count) = bytes_count {
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

        let result = thread.join();
        if let Err(_error) = result {
            println!("Progress bars error");
        }

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
