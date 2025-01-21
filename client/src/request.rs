use reqwest::Client;
use serde::Serialize;
use url::Url;

#[derive(Debug)]
pub struct Post<T>
where
    T: Serialize,
{
    pub url: Url,
    pub body: T,
}

impl<T> Post<T>
where
    T: Serialize,
{
    pub fn new(url: Url, body: T) -> Post<T> {
        Post { url, body }
    }
    pub async fn send(&self) -> Result<reqwest::Response, reqwest::Error> {
        let client = Client::new();
        client.post(self.url.clone()).json(&self.body).send().await
    }
}
