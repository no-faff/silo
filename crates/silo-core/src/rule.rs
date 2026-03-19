use crate::config::Rule;

pub fn find_matching_rule<'a>(rules: &'a [Rule], domain: &str) -> Option<&'a Rule> {
    rules.iter().find(|r| domain_matches(&r.domain, domain))
}

/// Supports exact matches ("github.com") and wildcard subdomains
/// ("*.google.com" matches "mail.google.com" but not "google.com").
pub fn domain_matches(pattern: &str, domain: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let domain = domain.to_lowercase();

    if let Some(suffix) = pattern.strip_prefix("*.") {
        domain.ends_with(&format!(".{suffix}"))
    } else {
        pattern == domain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BrowserRef;

    #[test]
    fn find_rule_returns_first_match() {
        let rules = vec![
            Rule {
                domain: "github.com".to_string(),
                browser: BrowserRef {
                    desktop_file: "firefox.desktop".to_string(),
                    args: None,
                },
            },
            Rule {
                domain: "*.github.com".to_string(),
                browser: BrowserRef {
                    desktop_file: "chrome.desktop".to_string(),
                    args: None,
                },
            },
        ];
        let result = find_matching_rule(&rules, "github.com");
        assert_eq!(result.unwrap().browser.desktop_file, "firefox.desktop");
    }

    #[test]
    fn find_rule_returns_none_when_no_match() {
        let rules = vec![Rule {
            domain: "github.com".to_string(),
            browser: BrowserRef {
                desktop_file: "firefox.desktop".to_string(),
                args: None,
            },
        }];
        assert!(find_matching_rule(&rules, "gitlab.com").is_none());
    }

    #[test]
    fn exact_match() {
        assert!(domain_matches("github.com", "github.com"));
    }

    #[test]
    fn exact_no_match() {
        assert!(!domain_matches("github.com", "gitlab.com"));
    }

    #[test]
    fn wildcard_matches_subdomain() {
        assert!(domain_matches("*.google.com", "mail.google.com"));
    }

    #[test]
    fn wildcard_matches_deep_subdomain() {
        assert!(domain_matches("*.google.com", "a.b.google.com"));
    }

    #[test]
    fn wildcard_does_not_match_bare_domain() {
        assert!(!domain_matches("*.google.com", "google.com"));
    }

    #[test]
    fn wildcard_does_not_match_unrelated() {
        assert!(!domain_matches("*.google.com", "evil-google.com"));
    }

    #[test]
    fn case_insensitive() {
        assert!(domain_matches("GitHub.com", "github.com"));
        assert!(domain_matches("github.com", "GitHub.COM"));
    }
}
