use bytes::Bytes;
use rayon::prelude::*;
use reqwest;

pub struct Web {
    client: reqwest::blocking::Client,
}

impl Web {
    pub fn new() -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .expect("Can't create reqwest client");
        Self { client }
    }

    pub fn get<'a>(&self, urls: &[&'a str]) -> Vec<(&'a str, reqwest::Result<Bytes>)> {
        let responses: Vec<(&str, reqwest::Result<Bytes>)> = urls
            .par_iter()
            .map(|url| {
                let bytes = self
                    .client
                    .get(*url)
                    .send()
                    .and_then(|response| response.bytes());
                (*url, bytes)
            })
            .collect();

        responses
    }
}
