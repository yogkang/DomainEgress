#[path = "../src/services/static_http.rs"]
mod static_http;
use static_http::{Manager, ServiceConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let artifact = PathBuf::from(std::env::args().nth(1).expect("请指定工件输出目录"));
    std::fs::create_dir_all(&artifact)?;
    let artifact = std::fs::canonicalize(artifact)?;
    let a = artifact.join("root-a");
    let b = artifact.join("root-b");
    std::fs::create_dir_all(&a)?;
    std::fs::create_dir_all(&b)?;
    std::fs::write(a.join("index.html"), "root-a")?;
    std::fs::write(b.join("index.html"), "root-b")?;
    std::fs::write(a.join("中文.txt"), "hello 中文")?;
    std::fs::write(a.join("large.txt"), "0123456789")?;
    std::fs::write(artifact.join("secret.txt"), "private")?;
    #[cfg(unix)]
    {
        let _ = std::fs::remove_file(a.join("escape.txt"));
        std::os::unix::fs::symlink(artifact.join("secret.txt"), a.join("escape.txt"))?;
    }
    let _ = std::fs::remove_file(artifact.join("services.json"));
    let manager = Manager::with_storage(artifact.join("services.json"));
    let free = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = free.local_addr()?.port();
    drop(free);
    let config = |root: &PathBuf| ServiceConfig {
        port,
        host: "127.0.0.1".into(),
        root: root.display().to_string(),
    };
    manager.save(config(&a))?;
    manager.start(port)?;
    let client = reqwest::Client::builder().no_proxy().build()?;
    let base = format!("http://127.0.0.1:{port}");
    let mut checks = Vec::<serde_json::Value>::new();
    for (name, path, expected, body) in [
        ("index", "/", 200, Some("root-a")),
        (
            "unicode",
            "/%E4%B8%AD%E6%96%87.txt",
            200,
            Some("hello 中文"),
        ),
        ("missing", "/missing.txt", 404, None),
        ("encoded traversal", "/%2e%2e%2fsecret.txt", 400, None),
        ("invalid escape", "/%GG", 400, None),
        ("symlink escape", "/escape.txt", 403, None),
    ] {
        let response = client.get(format!("{base}{path}")).send().await?;
        let status = response.status().as_u16();
        let text = response.text().await?;
        assert_eq!(status, expected, "{name}: {text}");
        if let Some(body) = body {
            assert_eq!(text, body);
        }
        checks.push(serde_json::json!({"name":name,"passed":true,"status":status}));
    }
    std::fs::create_dir_all(a.join("docs"))?;
    std::fs::write(a.join("docs/index.html"), "<link href=style.css>")?;
    std::fs::write(a.join("docs/style.css"), "body{}")?;
    let no_redirect = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let redirect = no_redirect.get(format!("{base}/docs?q=1")).send().await?;
    assert_eq!(redirect.status(), 308);
    assert_eq!(redirect.headers()["location"], "/docs/?q=1");
    let redirect = no_redirect.get(format!("{base}//docs")).send().await?;
    assert_eq!(redirect.status(), 308);
    assert_eq!(redirect.headers()["location"], "/docs/");
    assert_eq!(
        client
            .get(format!("{base}/docs/style.css"))
            .send()
            .await?
            .text()
            .await?,
        "body{}"
    );
    checks.push(
        serde_json::json!({"name":"directory redirect and query preservation","passed":true}),
    );
    let response = client.head(format!("{base}/large.txt")).send().await?;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-length"], "10");
    assert_eq!(response.text().await?, "");
    checks.push(serde_json::json!({"name":"HEAD","passed":true}));
    let response = client
        .get(format!("{base}/large.txt"))
        .header("Range", "bytes=2-5")
        .send()
        .await?;
    assert_eq!(response.status(), 206);
    assert_eq!(response.text().await?, "2345");
    checks.push(serde_json::json!({"name":"Range","passed":true}));
    assert_eq!(client.post(format!("{base}/")).send().await?.status(), 405);
    checks.push(serde_json::json!({"name":"method restriction","passed":true}));
    manager.save(config(&b))?;
    assert_eq!(
        client.get(format!("{base}/")).send().await?.text().await?,
        "root-b"
    );
    checks.push(serde_json::json!({"name":"live root switch","passed":true}));
    let conflict = Manager::with_storage(artifact.join("conflict.json"));
    conflict.save(config(&a))?;
    assert!(conflict.start(port).is_err());
    checks.push(serde_json::json!({"name":"port collision","passed":true}));
    assert!(manager.save(config(&artifact.join("missing"))).is_err());
    assert_eq!(manager.snapshot().services.len(), 1);
    let restored = Manager::with_storage(artifact.join("services.json"));
    assert_eq!(
        restored.snapshot().services[0].root,
        b.display().to_string()
    );
    assert!(restored.snapshot().running.is_empty());
    checks.push(serde_json::json!({"name":"persistence and one root per port","passed":true}));
    let free = std::net::TcpListener::bind("127.0.0.1:0")?;
    let second_port = free.local_addr()?.port();
    drop(free);
    manager.save(ServiceConfig {
        port: second_port,
        host: "127.0.0.1".into(),
        root: a.display().to_string(),
    })?;
    manager.start(second_port)?;
    assert_eq!(
        client
            .get(format!("http://127.0.0.1:{second_port}/"))
            .send()
            .await?
            .text()
            .await?,
        "root-a"
    );
    assert_eq!(
        client.get(format!("{base}/")).send().await?.text().await?,
        "root-b"
    );
    assert_eq!(manager.snapshot().running.len(), 2);
    manager.remove(second_port).await?;
    checks.push(serde_json::json!({"name":"independent ports and roots","passed":true}));
    let logs = manager.snapshot().logs;
    assert!(logs.len() >= 10);
    assert!(logs.iter().all(|log| log.client.starts_with("127.0.0.1:")));
    manager.clear();
    assert!(manager.snapshot().logs.is_empty());
    manager.stop(port).await;
    assert!(std::net::TcpStream::connect(format!("127.0.0.1:{port}")).is_err());
    assert!(
        client.get(format!("{base}/")).send().await.is_err(),
        "停止后已有连接仍可访问"
    );
    manager.start(port)?;
    manager.remove(port).await?;
    assert!(manager.snapshot().services.is_empty());
    checks.push(serde_json::json!({"name":"logs stop restart delete","passed":true}));
    let report = serde_json::json!({"passed":checks.len(),"checks":checks,"logs":logs});
    std::fs::write(
        artifact.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
