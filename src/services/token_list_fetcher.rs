use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use alloy::{primitives::Address, transports::http::Client};
use futures::{stream, StreamExt};
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::{
    config::constants::TOKEN_FETCH_CONCURRENCY,
    domain::{EvmNetwork, Token},
    services::errors::FetcherError,
};

const CACHE_TTL: Duration = Duration::from_secs(3600 * 5); // 5 hours

struct CachedTokenList {
    fetched_at: Instant,
    list: HashMap<u64, HashSet<Address>>,
}

pub struct TokenListFetcher {
    cache: RwLock<HashMap<String, CachedTokenList>>,
    in_flight: RwLock<HashSet<String>>,
    client: Client,
    ttl: Duration,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    tokens: Vec<Token>,
}

impl TokenListFetcher {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            client: Client::new(),
            ttl: CACHE_TTL,
            in_flight: RwLock::new(HashSet::new()),
        }
    }

    pub async fn get_tokens(
        &self,
        urls: &[String],
        network: EvmNetwork,
    ) -> Result<HashSet<Address>, FetcherError> {
        let uncached_urls = self.get_uncached_urls(urls).await;

        // fetch uncached lists
        if !uncached_urls.is_empty() {
            {
                // flag fetching urls
                let mut in_flight = self.in_flight.write().await;
                in_flight.extend(urls.iter().cloned());
            }

            let result = self.fetch_and_cache(&uncached_urls).await;

            {
                // unflag fetching urls
                let mut in_flight = self.in_flight.write().await;
                for url in urls {
                    in_flight.remove(url);
                }
            }

            result?;
        }

        let from_cache = self.collect_from_cache(urls, network).await;
        Ok(from_cache)
    }

    async fn fetch_and_cache(&self, urls: &[String]) -> Result<(), FetcherError> {
        let result: Vec<(String, Result<ApiResponse, FetcherError>)> =
            stream::iter(urls.iter().cloned())
                .map(move |url| {
                    let client = self.client.clone();
                    async move {
                        let response = Self::fetch_list(&client, &url).await;
                        (url, response)
                    }
                })
                .buffer_unordered(TOKEN_FETCH_CONCURRENCY)
                .collect()
                .await;

        for (_, response) in &result {
            if let Err(err) = response {
                return Err(err.clone());
            }
        }

        let mut mapped_by_url: HashMap<String, HashMap<u64, HashSet<Address>>> = HashMap::new();
        for (url, response) in result {
            if let Ok(api_resp) = response {
                let mut map_by_chain: HashMap<u64, HashSet<Address>> = HashMap::new();

                for token in api_resp.tokens {
                    map_by_chain
                        .entry(token.chain_id)
                        .or_default()
                        .insert(token.address);
                }

                if !map_by_chain.is_empty() {
                    mapped_by_url.insert(url, map_by_chain);
                }
            }
        }

        let loaded_urls: Vec<&String> = mapped_by_url.keys().collect();
        tracing::info!(lists = ?loaded_urls, "token lists loaded");

        let mut cache = self.cache.write().await;
        for (url, result) in mapped_by_url {
            cache.insert(
                url,
                CachedTokenList {
                    fetched_at: Instant::now(),
                    list: result,
                },
            );
        }

        Ok(())
    }

    async fn fetch_list(client: &Client, url: &String) -> Result<ApiResponse, FetcherError> {
        client
            .get(url)
            .send()
            .await
            .map_err(|err| FetcherError::UnableToLoadList(url.clone(), err.to_string()))?
            .json()
            .await
            .map_err(|err| FetcherError::UnableToLoadList(url.clone(), err.to_string()))
    }

    async fn collect_from_cache(&self, urls: &[String], network: EvmNetwork) -> HashSet<Address> {
        let cached_lists = self.cache.read().await;

        let mut result: HashSet<Address> = HashSet::new();
        for url in urls {
            if let Some(cached) = cached_lists.get(url) {
                if let Some(cached_by_chain) = cached.list.get(&network.chain_id()) {
                    result.extend(cached_by_chain.iter().cloned());
                }
            }
        }

        result
    }

    async fn get_uncached_urls(&self, urls: &[String]) -> Vec<String> {
        let cached_lists = self.cache.read().await;
        let in_flight = self.in_flight.read().await;
        let now = Instant::now();

        urls.iter()
            .filter(|url| {
                if in_flight.contains(*url) {
                    return false;
                }

                match cached_lists.get(*url) {
                    Some(cached_list) => now.duration_since(cached_list.fetched_at) >= self.ttl,
                    None => true,
                }
            })
            .cloned()
            .collect()
    }
}
