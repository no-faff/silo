use crate::config::Rule;
use crate::browser::BrowserEntry;

/// Finds the first rule whose pattern matches the given domain and path.
pub fn find_matching_rule<'a>(rules: &'a [Rule], domain: &str, path: &str) -> Option<&'a Rule> {
    rules.iter().find(|r| url_matches(domain, path, &r.pattern))
}

/// Returns true if the domain and path match a rule pattern.
///
/// Pattern syntax:
///   "github.com"          — exact domain (also matches www.github.com)
///   "*.google.com"        — any subdomain of google.com
///   "github.com/gist"     — domain + path prefix (/gist, /gist/abc, etc.)
///   "github.com/gist/*"   — domain + path wildcard (explicit)
///   "*.corp.com/internal"  — subdomain wildcard + path prefix
pub fn url_matches(domain: &str, path: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }

    let pattern = pattern.to_lowercase();
    let domain = domain.to_lowercase();
    let path = path.to_lowercase();

    match pattern.find('/') {
        None => {
            // Domain-only pattern
            domain_matches(&pattern, &domain)
        }
        Some(idx) => {
            let domain_part = &pattern[..idx];
            let path_part = &pattern[idx..]; // includes the leading /

            domain_matches(domain_part, &domain) && path_matches(&path, path_part)
        }
    }
}

/// Matches a domain against a pattern. Supports exact, www prefix and
/// wildcard subdomains.
fn domain_matches(pattern: &str, domain: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix("*.") {
        domain.ends_with(&format!(".{suffix}"))
    } else {
        domain == pattern || domain == format!("www.{pattern}")
    }
}

/// Matches a URL path against a path pattern.
/// Without wildcards, acts as a prefix match.
/// With a trailing /*, requires at least one segment after the prefix.
fn path_matches(url_path: &str, pattern: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix("/*") {
        // Wildcard: must match prefix and have something after it
        if let Some(rest) = url_path.strip_prefix(prefix) {
            rest.starts_with('/')
        } else {
            false
        }
    } else {
        // Prefix match on path boundary: exact or followed by /
        url_path == pattern
            || url_path.starts_with(&format!("{pattern}/"))
    }
}

/// Returns rules whose browser points to a desktop file + args combination
/// not present in the detected browsers list. Exception rules (browser: None)
/// are never stale.
pub fn find_stale_rules<'a>(rules: &'a [Rule], browsers: &[BrowserEntry]) -> Vec<&'a Rule> {
    rules
        .iter()
        .filter(|r| {
            if let Some(ref browser) = r.browser {
                !browsers.iter().any(|b| {
                    b.desktop_file == browser.desktop_file
                        && b.profile_args.as_deref() == browser.args.as_deref()
                })
            } else {
                false // exception rules are never stale
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BrowserRef;

    fn rule(pattern: &str) -> Rule {
        Rule {
            pattern: pattern.to_string(),
            browser: Some(BrowserRef {
                desktop_file: "firefox.desktop".to_string(),
                args: None,
            }),
        }
    }

    fn exception_rule(pattern: &str) -> Rule {
        Rule {
            pattern: pattern.to_string(),
            browser: None,
        }
    }

    // -- domain matching --

    #[test]
    fn exact_match() {
        assert!(url_matches("github.com", "/", "github.com"));
    }

    #[test]
    fn exact_no_match() {
        assert!(!url_matches("gitlab.com", "/", "github.com"));
    }

    #[test]
    fn www_prefix_matches() {
        assert!(url_matches("www.github.com", "/", "github.com"));
    }

    #[test]
    fn www_prefix_does_not_match_reverse() {
        // Pattern "www.github.com" should not match bare "github.com"
        assert!(!url_matches("github.com", "/", "www.github.com"));
    }

    #[test]
    fn wildcard_matches_subdomain() {
        assert!(url_matches("mail.google.com", "/", "*.google.com"));
    }

    #[test]
    fn wildcard_matches_deep_subdomain() {
        assert!(url_matches("a.b.google.com", "/", "*.google.com"));
    }

    #[test]
    fn wildcard_does_not_match_bare_domain() {
        assert!(!url_matches("google.com", "/", "*.google.com"));
    }

    #[test]
    fn wildcard_does_not_match_unrelated() {
        assert!(!url_matches("evil-google.com", "/", "*.google.com"));
    }

    #[test]
    fn case_insensitive() {
        assert!(url_matches("GitHub.com", "/", "github.com"));
        assert!(url_matches("github.com", "/", "GitHub.COM"));
    }

    // -- path matching --

    #[test]
    fn path_prefix_matches() {
        assert!(url_matches("github.com", "/gist", "github.com/gist"));
    }

    #[test]
    fn path_prefix_matches_subpath() {
        assert!(url_matches("github.com", "/gist/abc", "github.com/gist"));
    }

    #[test]
    fn path_prefix_does_not_match_similar() {
        assert!(!url_matches("github.com", "/gists", "github.com/gist"));
    }

    #[test]
    fn path_wildcard_matches() {
        assert!(url_matches("github.com", "/gist/abc", "github.com/gist/*"));
    }

    #[test]
    fn path_wildcard_does_not_match_bare() {
        assert!(!url_matches("github.com", "/gist", "github.com/gist/*"));
    }

    #[test]
    fn combined_wildcard_domain_and_path() {
        assert!(url_matches("app.corp.com", "/internal/page", "*.corp.com/internal"));
    }

    // -- find_matching_rule --

    #[test]
    fn find_rule_returns_first_match() {
        let rules = vec![rule("github.com"), rule("*.github.com")];
        let result = find_matching_rule(&rules, "github.com", "/");
        assert!(result.is_some());
        assert_eq!(result.unwrap().pattern, "github.com");
    }

    #[test]
    fn find_rule_returns_none_when_no_match() {
        let rules = vec![rule("github.com")];
        assert!(find_matching_rule(&rules, "gitlab.com", "/").is_none());
    }

    #[test]
    fn find_rule_matches_path() {
        let rules = vec![rule("github.com/gist")];
        assert!(find_matching_rule(&rules, "github.com", "/gist/abc").is_some());
        assert!(find_matching_rule(&rules, "github.com", "/pulls").is_none());
    }

    // -- exception rules --

    #[test]
    fn exception_rule_found() {
        let rules = vec![exception_rule("example.com")];
        let result = find_matching_rule(&rules, "example.com", "/");
        assert!(result.is_some());
        assert!(result.unwrap().browser.is_none());
    }

    // -- stale detection --

    #[test]
    fn stale_rule_detected() {
        let rules = vec![rule("example.com")];
        let browsers: Vec<BrowserEntry> = vec![];
        let stale = find_stale_rules(&rules, &browsers);
        assert_eq!(stale.len(), 1);
    }

    #[test]
    fn non_stale_rule_not_flagged() {
        let rules = vec![rule("example.com")];
        let browsers = vec![BrowserEntry {
            desktop_file: "firefox.desktop".to_string(),
            display_name: "Firefox".to_string(),
            icon: "firefox".to_string(),
            profile_args: None,
            exec: "firefox %u".to_string(),
        }];
        let stale = find_stale_rules(&rules, &browsers);
        assert!(stale.is_empty());
    }

    #[test]
    fn exception_rule_never_stale() {
        let rules = vec![exception_rule("example.com")];
        let browsers: Vec<BrowserEntry> = vec![];
        let stale = find_stale_rules(&rules, &browsers);
        assert!(stale.is_empty());
    }
}
