use crate::runtimes::Runtime;

struct RuntimeDef {
    id: &'static str,
    name: &'static str,
    endoflife_id: &'static str,
    changelog_pattern: ChangelogPattern,
}

enum ChangelogPattern {
    Format(&'static str),
    None,
}

impl Runtime for RuntimeDef {
    fn id(&self) -> &str {
        self.id
    }

    fn display_name(&self) -> &str {
        self.name
    }

    fn endoflife_id(&self) -> &str {
        self.endoflife_id
    }

    fn changelog_url(&self, cycle: &str) -> Option<String> {
        match &self.changelog_pattern {
            ChangelogPattern::Format(fmt) => Some(fmt.replace("{cycle}", cycle)),
            ChangelogPattern::None => None,
        }
    }
}

pub fn all_runtimes() -> Vec<Box<dyn Runtime>> {
    vec![
        Box::new(RuntimeDef {
            id: "python",
            name: "Python",
            endoflife_id: "python",
            changelog_pattern: ChangelogPattern::Format(
                "https://docs.python.org/3/whatsnew/{cycle}.html",
            ),
        }),
        Box::new(RuntimeDef {
            id: "go",
            name: "Go",
            endoflife_id: "go",
            changelog_pattern: ChangelogPattern::Format("https://go.dev/doc/go{cycle}"),
        }),
        Box::new(RuntimeDef {
            id: "rust",
            name: "Rust",
            endoflife_id: "rust",
            changelog_pattern: ChangelogPattern::None,
        }),
        Box::new(RuntimeDef {
            id: "nodejs",
            name: "Node.js",
            endoflife_id: "nodejs",
            changelog_pattern: ChangelogPattern::None,
        }),
        Box::new(RuntimeDef {
            id: "ruby",
            name: "Ruby",
            endoflife_id: "ruby",
            changelog_pattern: ChangelogPattern::None,
        }),
        Box::new(RuntimeDef {
            id: "php",
            name: "PHP",
            endoflife_id: "php",
            changelog_pattern: ChangelogPattern::None,
        }),
        Box::new(RuntimeDef {
            id: "java",
            name: "Eclipse Temurin (Java)",
            endoflife_id: "eclipse-temurin",
            changelog_pattern: ChangelogPattern::None,
        }),
        Box::new(RuntimeDef {
            id: "dotnet",
            name: ".NET",
            endoflife_id: "dotnet",
            changelog_pattern: ChangelogPattern::Format(
                "https://learn.microsoft.com/en-us/dotnet/core/whats-new/dotnet-{cycle}",
            ),
        }),
        Box::new(RuntimeDef {
            id: "deno",
            name: "Deno",
            endoflife_id: "deno",
            changelog_pattern: ChangelogPattern::None,
        }),
        Box::new(RuntimeDef {
            id: "bun",
            name: "Bun",
            endoflife_id: "bun",
            changelog_pattern: ChangelogPattern::None,
        }),
        Box::new(RuntimeDef {
            id: "elixir",
            name: "Elixir",
            endoflife_id: "elixir",
            changelog_pattern: ChangelogPattern::None,
        }),
        Box::new(RuntimeDef {
            id: "kotlin",
            name: "Kotlin",
            endoflife_id: "kotlin",
            changelog_pattern: ChangelogPattern::None,
        }),
        Box::new(RuntimeDef {
            id: "perl",
            name: "Perl",
            endoflife_id: "perl",
            changelog_pattern: ChangelogPattern::None,
        }),
        Box::new(RuntimeDef {
            id: "scala",
            name: "Scala",
            endoflife_id: "scala",
            changelog_pattern: ChangelogPattern::None,
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_runtimes_have_unique_ids() {
        let runtimes = all_runtimes();
        let mut ids: Vec<&str> = runtimes.iter().map(|r| r.id()).collect();
        let len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), len, "duplicate runtime IDs found");
    }

    #[test]
    fn test_python_changelog_url() {
        let runtimes = all_runtimes();
        let python = runtimes.iter().find(|r| r.id() == "python").unwrap();
        assert_eq!(
            python.changelog_url("3.14"),
            Some("https://docs.python.org/3/whatsnew/3.14.html".to_string())
        );
    }

    #[test]
    fn test_rust_has_no_changelog_pattern() {
        let runtimes = all_runtimes();
        let rust = runtimes.iter().find(|r| r.id() == "rust").unwrap();
        assert_eq!(rust.changelog_url("1.80"), None);
    }
}
