use http_body_util::{BodyExt, Empty};
use hyper::{Request, body::Bytes};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use rune::{Any, ContextError, Module};
use std::time::Duration;

#[derive(Any, Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    body: String,
}

impl HttpResponse {
    #[rune::function(instance)]
    fn text(&self) -> String {
        self.body.clone()
    }

    #[rune::function(instance)]
    fn status(&self) -> u16 {
        self.status.clone()
    }
}

#[rune::function]
async fn sleep(seconds: f64) -> rune::support::Result<()> {
    if seconds < 0.0 {
        return Err(rune::support::Error::msg(
            "sleep duration cannot be negative",
        ));
    }
    tokio::time::sleep(Duration::from_secs_f64(seconds)).await;
    Ok(())
}

#[rune::function]
async fn get(url: String) -> rune::support::Result<HttpResponse> {
    let https = HttpsConnectorBuilder::new()
        .with_native_roots()
        .map_err(|e| rune::support::Error::msg(e.to_string()))?
        .https_only()
        .enable_http1()
        .build();

    let client = Client::builder(TokioExecutor::new()).build(https);

    let req = Request::builder()
        .uri(&url)
        .body(Empty::<Bytes>::new())
        .map_err(|e| rune::support::Error::msg(e.to_string()))?;

    let resp = client
        .request(req)
        .await
        .map_err(|e| rune::support::Error::msg(e.to_string()))?;

    let status = resp.status().as_u16();
    let body_bytes = resp
        .into_body()
        .collect()
        .await
        .map_err(|e| rune::support::Error::msg(e.to_string()))?
        .to_bytes();

    let body = String::from_utf8_lossy(&body_bytes).to_string();
    Ok(HttpResponse { status, body })
}

pub fn our_tool() -> Result<rune::Context, ContextError> {
    let mut ctx = rune::Context::with_default_modules()?;
    let mut root = Module::new();
    root.ty::<HttpResponse>()?;
    root.function_meta(HttpResponse::text)?;
    root.function_meta(HttpResponse::status)?;
    root.function_meta(sleep)?;
    let mut http = Module::with_item(["http"])?;
    http.function_meta(get)?;
    ctx.install(root)?;
    ctx.install(http)?;
    Ok(ctx)
}
