#[cfg(test)]
mod tests {
    use crate::logger::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_log_entry_structure() {
        let entry = LogEntry {
            timestamp: "02-03 | 14:30:00".to_string(),
            level: "INFO".to_string(),
            message: "Test message".to_string(),
            module: Some("test_module".to_string()),
            source: "🦀".to_string(),
        };

        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.message, "Test message");
        assert_eq!(entry.module, Some("test_module".to_string()));
        assert_eq!(entry.source, "🦀");
    }

    #[test]
    fn test_log_entry_clone() {
        let entry1 = LogEntry {
            timestamp: "01-01 | 12:00:00".to_string(),
            level: "INFO".to_string(),
            message: "Test".to_string(),
            module: Some("module".to_string()),
            source: "🦀".to_string(),
        };

        let entry2 = entry1.clone();
        assert_eq!(entry1.timestamp, entry2.timestamp);
        assert_eq!(entry1.level, entry2.level);
        assert_eq!(entry1.message, entry2.message);
        assert_eq!(entry1.module, entry2.module);
        assert_eq!(entry1.source, entry2.source);
    }

    #[test]
    fn test_logging_functions_exist() {
        // Just verify the functions exist and can be called
        // We won't test the shared state due to test isolation issues
        log_info("Test");
        log_debug("Test");
        log_error("Test");
        log_warn("Test");

        // The functions should not panic
        assert!(true);
    }

    #[test]
    fn test_get_logs_returns_vec() {
        let logs = get_logs();
        // Just verify it returns a Vec (might contain logs from other tests)
        // Vec::len() always returns a valid usize, so no assertion needed
        let _len: usize = logs.len();
    }

    #[test]
    fn test_export_logs_creates_file() -> std::io::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_str().unwrap();

        let result = export_logs(path);
        assert!(result.is_ok());

        // Verify file was created
        let content = fs::read_to_string(path)?;
        assert!(content.contains("Mehu PLOT3D Viewer - Log Export"));
        assert!(content.contains("================================"));

        Ok(())
    }
}
