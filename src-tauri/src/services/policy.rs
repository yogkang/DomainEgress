use std::net::IpAddr;
pub fn allowed(mode: &str, rules: &[String], target: &str) -> bool {
    let t = target.trim_end_matches('.').to_lowercase();
    let hit = if t.parse::<IpAddr>().is_ok() {
        rules.iter().any(|r| r.eq_ignore_ascii_case(&t))
    } else {
        rules.iter().any(|r| {
            let x = r
                .trim()
                .trim_end_matches('.')
                .trim_start_matches('.')
                .to_lowercase();
            if let Some(base) = x.strip_prefix("*.") {
                t.ends_with(&format!(".{base}")) && !t[..t.len() - base.len() - 1].contains('.')
            } else {
                t == x || t.ends_with(&format!(".{x}"))
            }
        })
    };
    if mode == "blacklist" {
        !hit
    } else {
        hit
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modes() {
        assert!(allowed(
            "whitelist",
            &["example.com".into()],
            "a.example.com"
        ));
        assert!(allowed(
            "whitelist",
            &["example.com".into()],
            "a.b.example.com"
        ));
        assert!(allowed(
            "whitelist",
            &[".example.com".into()],
            "a.b.example.com"
        ));
        assert!(allowed(
            "whitelist",
            &["*.example.com".into()],
            "a.example.com"
        ));
        assert!(!allowed(
            "whitelist",
            &["*.example.com".into()],
            "a.b.example.com"
        ));
        assert!(!allowed(
            "whitelist",
            &["*.example.com".into()],
            "example.com"
        ));
        assert!(!allowed(
            "whitelist",
            &["example.com".into()],
            "badexample.com"
        ));
        assert!(!allowed(
            "blacklist",
            &["example.com".into()],
            "a.example.com"
        ));
        assert!(!allowed(
            "blacklist",
            &["example.com".into()],
            "example.com"
        ));
        assert!(!allowed(
            "blacklist",
            &["example.com".into()],
            "a.b.example.com"
        ));
        assert!(allowed(
            "blacklist",
            &["*.example.com".into()],
            "a.b.example.com"
        ));
        assert!(allowed("blacklist", &[], "anything.com"));
    }
}
