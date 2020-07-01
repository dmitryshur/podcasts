use bytes::Bytes;
#[cfg(not(test))]
use rayon::prelude::*;
#[cfg(not(test))]
use reqwest;

#[cfg(test)]
use std::io::Read;

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

    #[cfg(not(test))]
    pub fn get<'a>(&self, urls: &[&'a str]) -> Vec<(&'a str, reqwest::Result<Bytes>)> {
        let responses: Vec<(&str, reqwest::Result<Bytes>)> = urls
            .par_iter()
            .map(|url| {
                let bytes = self.client.get(*url).send().and_then(|response| response.bytes());
                (*url, bytes)
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
