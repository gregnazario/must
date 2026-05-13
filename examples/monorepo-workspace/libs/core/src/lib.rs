pub fn greet(name: &str) -> String {
    format!("hello, {}", name)
}

pub fn version() -> &'static str {
    "2.0.0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(greet("world"), "hello, world");
    }

    #[test]
    fn test_version() {
        assert_eq!(version(), "2.0.0");
    }
}
