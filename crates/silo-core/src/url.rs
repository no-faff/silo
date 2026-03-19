/// Lowercased domain from a URL, or None if unparseable.
pub fn extract_domain(input: &str) -> Option<String> {
    let parsed = url::Url::parse(input).ok()?;
    let host = parsed.host_str()?;
    Some(host.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_https_url() {
        assert_eq!(
            extract_domain("https://github.com/no-faff/silo"),
            Some("github.com".to_string())
        );
    }

    #[test]
    fn http_url() {
        assert_eq!(
            extract_domain("http://example.com/page?q=1"),
            Some("example.com".to_string())
        );
    }

    #[test]
    fn url_with_port() {
        assert_eq!(
            extract_domain("https://localhost:8080/path"),
            Some("localhost".to_string())
        );
    }

    #[test]
    fn url_with_subdomain() {
        assert_eq!(
            extract_domain("https://mail.google.com"),
            Some("mail.google.com".to_string())
        );
    }

    #[test]
    fn uppercase_domain_lowercased() {
        assert_eq!(
            extract_domain("https://GitHub.COM/page"),
            Some("github.com".to_string())
        );
    }

    #[test]
    fn garbage_input() {
        assert_eq!(extract_domain("not a url"), None);
    }

    #[test]
    fn empty_input() {
        assert_eq!(extract_domain(""), None);
    }
}
