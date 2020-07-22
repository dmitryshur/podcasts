use crate::Errors;
use bytes::Bytes;
#[cfg(not(test))]
use rayon::prelude::*;
#[cfg(not(test))]
use reqwest;

use reqwest::StatusCode;
#[cfg(test)]
use std::io::Read;

pub struct Web {
    client: reqwest::blocking::Client,
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
        let responses: Vec<(&str, Result<Bytes, Errors>)> = urls
            .par_iter()
            .map(|url| {
                println!("Fetching URL {}", *url);

                let bytes = self.client.get(*url).send();
                return match bytes {
                    Ok(response) => {
                        if response.status() == StatusCode::NOT_FOUND {
                            return (*url, Err(Errors::NotFound((*url).to_string())));
                        }

                        let bytes = response.bytes();
                        if let Ok(bytes) = bytes {
                            return (*url, Ok(bytes));
                        }

                        (*url, Err(Errors::Network(bytes.err().unwrap())))
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
