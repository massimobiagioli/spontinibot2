pub fn version() -> &'static str {
    "ingest-core 0.1.0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_version_when_called() {
        let result = version();
        assert_eq!(result, "ingest-core 0.1.0");
    }
}
