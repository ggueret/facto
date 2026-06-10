/// Normalize a Python package name per PEP 503:
/// lowercase, replace any run of [-_.] with a single hyphen.
pub fn pypi_normalize(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut in_separator = false;

    for c in name.chars() {
        if c == '-' || c == '_' || c == '.' {
            if !in_separator {
                result.push('-');
                in_separator = true;
            }
            // Skip additional consecutive separators
        } else {
            in_separator = false;
            // Push lowercase char
            for lc in c.to_lowercase() {
                result.push(lc);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pypi_normalize_basic() {
        assert_eq!(pypi_normalize("My_Package"), "my-package");
        assert_eq!(pypi_normalize("some.thing"), "some-thing");
        assert_eq!(pypi_normalize("UPPER-CASE"), "upper-case");
        assert_eq!(pypi_normalize("already-normalized"), "already-normalized");
        assert_eq!(pypi_normalize("CamelCase"), "camelcase");
    }

    #[test]
    fn test_pypi_normalize_consecutive_separators() {
        assert_eq!(
            pypi_normalize("multiple___underscores"),
            "multiple-underscores"
        );
        assert_eq!(pypi_normalize("dots...and___mixed"), "dots-and-mixed");
        assert_eq!(pypi_normalize("a_b.c-d"), "a-b-c-d");
        assert_eq!(pypi_normalize("a-_.-b"), "a-b");
    }

    #[test]
    fn test_pypi_normalize_edge_cases() {
        assert_eq!(pypi_normalize("a"), "a");
        assert_eq!(pypi_normalize(""), "");
        assert_eq!(pypi_normalize("_leading"), "-leading");
        assert_eq!(pypi_normalize("trailing_"), "trailing-");
    }
}
