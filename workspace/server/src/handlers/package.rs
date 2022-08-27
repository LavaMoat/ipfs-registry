use axum::{
    body::Bytes,
    extract::{Extension, TypedHeader},
    headers::ContentType,
    http::{StatusCode, uri::Scheme},
};

//use axum_macros::debug_handler;

use std::{sync::Arc, io::Cursor};
use tokio::sync::RwLock;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient, TryFromUri};
use url::Url;
use futures::TryStreamExt;

use ipfs_registry_core::{decompress, read_npm_package};
use crate::{State, Error, Result};

async fn ipfs_add(url: Url, data: Bytes) -> Result<String> {
    let client = new_ipfs_client(url)?;
    let data = Cursor::new(data);
    let add_res = client.add(data).await?;

    println!("{:#?}", add_res);

    let _pin_res = client.pin_add(&add_res.hash, true).await?;
    Ok(add_res.hash)
}

async fn ipfs_cat(url: Url, hash: &str) -> Result<Vec<u8>> {
    let client = new_ipfs_client(url)?;
    let res = client.cat(hash)
        .map_ok(|chunk| chunk.to_vec())
        .try_concat()
        .await?;
    Ok(res)
}

/// Create a new IPFS client from the config URL.
fn new_ipfs_client(url: Url) -> Result<IpfsClient> {
    let host = url.host_str()
        .ok_or(Error::InvalidHost(url.clone()))?;

    let port = url.port_or_known_default()
        .ok_or(Error::InvalidPort(url.clone()))?;

    let scheme = if url.scheme() == "http" {
        Scheme::HTTP
    } else if url.scheme() == "https" {
        Scheme::HTTPS
    } else {
       return Err(Error::InvalidScheme(url.scheme().to_owned())) 
    };

    Ok(IpfsClient::from_host_and_port(scheme, host, port)?)
}

pub(crate) struct PackageHandler;
impl PackageHandler {
    /// Create a new package.
    pub(crate) async fn put(
        Extension(state): Extension<Arc<RwLock<State>>>,
        TypedHeader(mime): TypedHeader<ContentType>,
        body: Bytes,
    ) -> std::result::Result<StatusCode, StatusCode> {

        // TODO: validate signature
        // TODO: ensure approval signatures

        let gzip: mime::Mime = "application/gzip".parse().unwrap();
        let gzip_ct = ContentType::from(gzip);

        if mime == gzip_ct {
            let contents = decompress(&body)
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            let descriptor = read_npm_package(&contents)
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            println!("{:#?}", descriptor);

            // TODO: store in the index

            let reader = state.read().await;
            let url = reader.config.ipfs.as_ref().unwrap().url.clone();
            drop(reader);

            let hash = ipfs_add(url, body)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            println!("{}", hash);

            /*
            let bytes = ipfs_cat(url, &hash)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            println!("{}", bytes.len());
            println!("{:#?}", bytes);
            */

            Ok(StatusCode::OK)

        } else {
            Err(StatusCode::BAD_REQUEST)
        }
    }
}
