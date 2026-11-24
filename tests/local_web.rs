#![allow(dead_code)]

#[path = "../src/adapters.rs"]
mod adapters;
#[path = "../src/issue.rs"]
mod issue;
#[path = "../src/prompt.rs"]
mod prompt;
#[path = "../src/settings.rs"]
mod settings;
#[path = "../src/web.rs"]
mod web;

use settings::Config;
use std::path::Path;

#[tokio::test]
async fn local_web_rejects_unauthenticated_and_wrong_host_requests() {
    let config = Config::load(Path::new("config")).expect("repository configuration parses");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("loopback test listener binds");
    let address = listener.local_addr().expect("listener address exists");
    let origin = format!("http://{address}");
    let (router, capability) = web::build_router(Path::new("config"), config, "en", origin)
        .expect("local web router builds from configured assets");
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("test web server runs");
    });

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("test client builds without following redirects");
    let base = format!("http://{address}");
    let unauthenticated = client
        .get(format!("{base}/app"))
        .send()
        .await
        .expect("unauthenticated request completes");
    assert_eq!(unauthenticated.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(unauthenticated.headers()["cache-control"], "no-store");
    assert!(
        unauthenticated
            .headers()
            .contains_key("content-security-policy")
    );

    let wrong_host = client
        .get(format!("{base}/app"))
        .header(reqwest::header::HOST, "untrusted.example")
        .send()
        .await
        .expect("wrong-host request completes");
    assert_eq!(
        wrong_host.status(),
        reqwest::StatusCode::MISDIRECTED_REQUEST
    );

    let capability_path = reqwest::Url::parse(capability.as_str())
        .expect("bootstrap URL parses")
        .path()
        .to_owned();
    let bootstrap = client
        .get(format!("{base}{capability_path}"))
        .send()
        .await
        .expect("one-use bootstrap request completes");
    assert_eq!(bootstrap.status(), reqwest::StatusCode::SEE_OTHER);
    let cookie = bootstrap.headers()[reqwest::header::SET_COOKIE]
        .to_str()
        .expect("bootstrap cookie is valid")
        .split(';')
        .next()
        .expect("cookie pair exists")
        .to_owned();

    let authenticated = client
        .get(format!("{base}/app"))
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .expect("session cookie opens the local page");
    assert_eq!(authenticated.status(), reqwest::StatusCode::OK);

    let cross_origin = client
        .post(format!("{base}/api/prepare"))
        .header(reqwest::header::COOKIE, &cookie)
        .header(reqwest::header::ORIGIN, "http://untrusted.example")
        .json(&serde_json::json!({
            "url": "https://github.com/owner/repository/issues/1",
            "agent_id": null
        }))
        .send()
        .await
        .expect("cross-origin request completes");
    assert_eq!(cross_origin.status(), reqwest::StatusCode::FORBIDDEN);

    let reused_capability = client
        .get(format!("{base}{capability_path}"))
        .send()
        .await
        .expect("reused bootstrap request completes");
    assert_eq!(
        reused_capability.status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    server.abort();
}
