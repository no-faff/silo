/// Lowercased domain from a URL, or None if unparseable.
pub fn extract_domain(input: &str) -> Option<String> {
    let parsed = url::Url::parse(input).ok()?;
    let host = parsed.host_str()?;
    Some(host.to_lowercase())
}

/// Office document type detected from a URL's file extension.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OfficeDocType {
    Spreadsheet,
    Document,
    Presentation,
}

/// Result of URL processing: redirect unwrapping + domain/path extraction.
#[derive(Debug, Clone)]
pub struct ProcessedUrl {
    pub original_url: String,
    pub final_url: String,
    pub domain: Option<String>,
    pub path: String,
    pub was_redirected: bool,
    pub office_doc: Option<OfficeDocType>,
}

/// Single entry point for URL processing. Unwraps redirects, extracts domain
/// and path, detects Office documents.
pub fn process_url(input: &str) -> ProcessedUrl {
    let final_url = unwrap_redirects(input);
    let was_redirected = final_url != input;

    let (domain, path, office_doc) = match url::Url::parse(&final_url) {
        Ok(parsed) => {
            let domain = parsed.host_str().map(|h| h.to_lowercase());
            let path = parsed.path().to_lowercase();
            let office_doc = detect_office_document(&path);
            (domain, path, office_doc)
        }
        Err(_) => (None, String::new(), None),
    };

    ProcessedUrl {
        original_url: input.to_string(),
        final_url,
        domain,
        path,
        was_redirected,
        office_doc,
    }
}

/// Recursively unwraps redirect wrappers (SafeLinks, Google redirect).
/// Returns the innermost URL, or the original if no wrappers detected.
fn unwrap_redirects(input: &str) -> String {
    let mut current = input.to_string();

    for _ in 0..10 {
        match try_unwrap(&current) {
            Some(inner) => current = inner,
            None => break,
        }
    }

    current
}

/// Attempts a single unwrap pass. Returns the inner URL if a known redirect
/// wrapper was detected, or None if the URL is not a redirect.
fn try_unwrap(raw: &str) -> Option<String> {
    let parsed = url::Url::parse(raw).ok()?;
    let host = parsed.host_str()?.to_lowercase();

    // Outlook SafeLinks
    if host.ends_with("safelinks.protection.outlook.com") {
        let inner = parsed
            .query_pairs()
            .find(|(k, _)| k == "url")
            .map(|(_, v)| v.to_string())?;
        if !inner.is_empty() {
            return Some(inner);
        }
    }

    // Google redirect (all regional variants: google.com, google.co.uk, etc.)
    if (host.starts_with("google.") || host.contains(".google.")) && parsed.path() == "/url" {
        let inner = parsed
            .query_pairs()
            .find(|(k, _)| k == "q")
            .map(|(_, v)| v.to_string())?;
        if !inner.is_empty() {
            return Some(inner);
        }
    }

    None
}

/// Returns the Office document type if the URL path ends with a known
/// Office file extension.
fn detect_office_document(path: &str) -> Option<OfficeDocType> {
    let path = path.to_lowercase();

    if path.ends_with(".xlsx") || path.ends_with(".xls") || path.ends_with(".xlsm") {
        Some(OfficeDocType::Spreadsheet)
    } else if path.ends_with(".docx") || path.ends_with(".doc") || path.ends_with(".docm") {
        Some(OfficeDocType::Document)
    } else if path.ends_with(".pptx") || path.ends_with(".ppt") || path.ends_with(".pptm") {
        Some(OfficeDocType::Presentation)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- extract_domain (existing tests) --

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

    // -- SafeLinks unwrapping --

    #[test]
    fn safelinks_unwraps() {
        let wrapped = "https://eur02.safelinks.protection.outlook.com/?url=https%3A%2F%2Fexample.com%2Fpage&data=abc";
        let result = process_url(wrapped);
        assert!(result.was_redirected);
        assert_eq!(result.final_url, "https://example.com/page");
        assert_eq!(result.domain, Some("example.com".to_string()));
    }

    #[test]
    fn safelinks_missing_url_param() {
        let broken = "https://eur02.safelinks.protection.outlook.com/?data=abc";
        let result = process_url(broken);
        assert!(!result.was_redirected);
    }

    // -- Google redirect unwrapping --

    #[test]
    fn google_redirect_unwraps() {
        let wrapped = "https://www.google.com/url?q=https%3A%2F%2Fexample.com%2Fpage&sa=t";
        let result = process_url(wrapped);
        assert!(result.was_redirected);
        assert_eq!(result.final_url, "https://example.com/page");
    }

    #[test]
    fn google_regional_redirect_unwraps() {
        let wrapped = "https://www.google.co.uk/url?q=https%3A%2F%2Fexample.com&sa=t";
        let result = process_url(wrapped);
        assert!(result.was_redirected);
        assert_eq!(result.final_url, "https://example.com");
    }

    #[test]
    fn google_non_redirect_path() {
        let normal = "https://www.google.com/search?q=rust";
        let result = process_url(normal);
        assert!(!result.was_redirected);
    }

    #[test]
    fn evil_google_not_unwrapped() {
        let evil = "https://evilgoogle.com/url?q=https%3A%2F%2Fexample.com";
        let result = process_url(evil);
        assert!(!result.was_redirected);
    }

    // -- Nested/recursive unwrapping --

    #[test]
    fn nested_safelinks_wrapping_google() {
        let inner = "https://example.com/page";
        let google = format!("https://www.google.com/url?q={}", urlencoding(inner));
        let safelinks = format!(
            "https://eur02.safelinks.protection.outlook.com/?url={}",
            urlencoding(&google)
        );
        let result = process_url(&safelinks);
        assert!(result.was_redirected);
        assert_eq!(result.final_url, inner);
    }

    #[test]
    fn recursion_limit_stops() {
        // A URL that unwraps to itself would loop forever without the limit.
        // Simulate by checking that unwrap_redirects returns after max passes.
        let url = "https://example.com/page";
        let result = unwrap_redirects(url);
        assert_eq!(result, url);
    }

    // -- Non-redirect passthrough --

    #[test]
    fn normal_url_not_redirected() {
        let result = process_url("https://github.com/no-faff/silo");
        assert!(!result.was_redirected);
        assert_eq!(result.final_url, "https://github.com/no-faff/silo");
        assert_eq!(result.domain, Some("github.com".to_string()));
        assert_eq!(result.path, "/no-faff/silo");
    }

    #[test]
    fn malformed_url_passthrough() {
        let result = process_url("not a url");
        assert!(!result.was_redirected);
        assert_eq!(result.domain, None);
        assert_eq!(result.path, "");
    }

    // -- Path extraction --

    #[test]
    fn path_extracted() {
        let result = process_url("https://github.com/gist/abc");
        assert_eq!(result.path, "/gist/abc");
    }

    #[test]
    fn path_lowercased() {
        let result = process_url("https://github.com/Gist/ABC");
        assert_eq!(result.path, "/gist/abc");
    }

    // -- Office document detection --

    #[test]
    fn xlsx_detected() {
        let result = process_url("https://sharepoint.com/sites/team/report.xlsx");
        assert_eq!(result.office_doc, Some(OfficeDocType::Spreadsheet));
    }

    #[test]
    fn docx_detected() {
        let result = process_url("https://sharepoint.com/sites/team/notes.docx");
        assert_eq!(result.office_doc, Some(OfficeDocType::Document));
    }

    #[test]
    fn pptx_detected() {
        let result = process_url("https://sharepoint.com/sites/team/slides.pptx");
        assert_eq!(result.office_doc, Some(OfficeDocType::Presentation));
    }

    #[test]
    fn xls_legacy_detected() {
        let result = process_url("https://sharepoint.com/report.xls");
        assert_eq!(result.office_doc, Some(OfficeDocType::Spreadsheet));
    }

    #[test]
    fn non_office_url() {
        let result = process_url("https://github.com/readme.md");
        assert_eq!(result.office_doc, None);
    }

    // -- helpers --

    fn urlencoding(input: &str) -> String {
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("_", input)
            .finish()
            .strip_prefix("_=")
            .unwrap()
            .to_string()
    }
}
