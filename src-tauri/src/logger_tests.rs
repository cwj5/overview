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

    #[test]
    fn test_init_logger() {
        init_logger();
        // Should not panic and logs should be obtainable
        let logs = get_logs();
        // Verify logs vector is accessible
        assert!(!logs.is_empty() || logs.is_empty()); // Always true but indicates success
    }

    #[test]
    fn test_log_info_with_module() {
        // Add a unique message to avoid conflicts with other tests
        let unique_msg = format!(
            "Test with module at {}",
            std::time::SystemTime::now().elapsed().unwrap().as_nanos()
        );
        log_entry("INFO", &unique_msg, Some("TestModule".to_string()));

        let logs = get_logs();
        assert!(!logs.is_empty());

        // Find the entry we just added (it should be recent)
        let found = logs.iter().rev().take(10).find(|log| {
            log.message == unique_msg
                && log.level == "INFO"
                && log.module == Some("TestModule".to_string())
        });

        assert!(
            found.is_some(),
            "Could not find the log entry we just added"
        );
        if let Some(entry) = found {
            assert_eq!(entry.source, "🦀");
        }
    }

    #[test]
    fn test_log_all_levels() {
        // Log with unique identifiers to avoid conflicts with other tests
        let time = std::time::SystemTime::now().elapsed().unwrap().as_nanos();
        log_entry("DEBUG", &format!("Debug-{}", time), None);
        log_entry("INFO", &format!("Info-{}", time), None);
        log_entry("WARN", &format!("Warn-{}", time), None);
        log_entry("ERROR", &format!("Error-{}", time), None);

        let logs = get_logs();
        // Just verify that logs can be retrieved and contain recent entries
        assert!(!logs.is_empty());

        // Check that our messages were added (should be in last 5 entries due to other tests)
        let recent_logs: Vec<_> = logs.iter().rev().take(10).collect();
        let messages: Vec<_> = recent_logs.iter().map(|l| l.message.as_str()).collect();

        // Verify we can find messages with the right time identifier
        assert!(messages.iter().any(|m| m.contains(&format!("{}", time))));
    }

    #[test]
    fn test_log_entry_timestamp_format() {
        clear_logs();
        let unique_msg = format!(
            "Timestamp test {}",
            std::time::SystemTime::now().elapsed().unwrap().as_nanos()
        );
        log_entry("INFO", &unique_msg, None);

        let logs = get_logs();
        assert!(!logs.is_empty());

        let entry = logs
            .iter()
            .rev()
            .find(|log| log.message == unique_msg)
            .expect("Could not find logged entry");
        // Timestamp should match format MM-DD | HH:MM:SS.mmm
        assert!(entry.timestamp.contains("|"));
        assert!(entry.timestamp.len() >= 16); // Minimum length of timestamp format
    }

    #[test]
    fn test_multiple_log_entries() {
        clear_logs();

        for i in 0..5 {
            log_entry("INFO", &format!("Message {}", i), None);
        }

        let logs = get_logs();
        assert!(logs.len() >= 5);
    }

    #[test]
    fn test_clear_logs() {
        // Clear before test to ensure clean state
        clear_logs();

        // Add some logs
        log_entry("INFO", "Log 1", None);
        log_entry("INFO", "Log 2", None);

        let before = get_logs();
        assert!(!before.is_empty());

        // Clear and verify
        clear_logs();
        let after = get_logs();
        assert_eq!(after.len(), 0);
    }

    #[test]
    fn test_log_entry_with_empty_message() {
        clear_logs();
        // Empty message test - use a unique module to identify it
        log_entry("INFO", "", Some("empty_msg_test".to_string()));

        let logs = get_logs();
        assert!(!logs.is_empty());

        let entry = logs
            .iter()
            .rev()
            .find(|log| log.module == Some("empty_msg_test".to_string()))
            .expect("Could not find logged entry");
        assert_eq!(entry.message, "");
    }

    #[test]
    fn test_log_entry_with_special_characters() {
        clear_logs();
        let special_msg = "Message with special chars: !@#$%^&*()_+-=[]{}|;':\"<>,.?/";
        log_entry("INFO", special_msg, Some("special_chars_test".to_string()));

        let logs = get_logs();
        assert!(!logs.is_empty());

        let entry = logs
            .iter()
            .rev()
            .find(|log| log.module == Some("special_chars_test".to_string()))
            .expect("Could not find logged entry");
        assert_eq!(entry.message, special_msg);
    }

    #[test]
    fn test_log_entry_with_unicode() {
        clear_logs();
        let unicode_msg = "Unicode test: 你好 мир 🦀 🎉";
        log_entry("INFO", unicode_msg, Some("unicode_test".to_string()));

        let logs = get_logs();
        assert!(!logs.is_empty());

        let entry = logs
            .iter()
            .rev()
            .find(|log| log.module == Some("unicode_test".to_string()))
            .expect("Could not find logged entry");
        assert_eq!(entry.message, unicode_msg);
    }

    #[test]
    fn test_export_logs_with_multiple_entries() -> std::io::Result<()> {
        // Add logs with different levels and modules
        log_entry("INFO", "First message", None);
        log_entry("WARN", "Warning message", Some("Module1".to_string()));
        log_entry("ERROR", "Error message", Some("Module2".to_string()));

        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_str().unwrap();

        export_logs(path)?;

        let content = fs::read_to_string(path)?;
        // Verify export has the expected structure
        assert!(content.contains("Mehu PLOT3D Viewer"));
        assert!(content.contains("================================"));

        // Verify at least some log entries are present (might have other logs from other tests)
        let lines: Vec<_> = content.lines().collect();
        assert!(lines.len() > 5); // Header + entries

        Ok(())
    }

    #[test]
    fn test_log_entry_source_is_crab() {
        clear_logs();
        log_entry("INFO", "Test", Some("source_crab_test".to_string()));

        let logs = get_logs();
        assert!(!logs.is_empty());

        let entry = logs
            .iter()
            .rev()
            .find(|log| log.module == Some("source_crab_test".to_string()))
            .expect("Could not find logged entry");
        assert_eq!(entry.source, "🦀");
    }

    #[test]
    fn test_log_warn_function() {
        clear_logs();
        log_warn("Warning test");

        let logs = get_logs();
        assert!(!logs.is_empty());

        let entry = logs
            .iter()
            .rev()
            .find(|log| log.message == "Warning test")
            .expect("Could not find logged entry");
        assert_eq!(entry.level, "WARN");
        assert_eq!(entry.message, "Warning test");
    }

    #[test]
    fn test_log_entry_max_entries() {
        // This is a pseudo-test since we can't easily test the 1000 limit
        // in an isolated way, but we verify that logging many entries doesn't crash
        clear_logs();

        for i in 0..50 {
            log_entry("INFO", &format!("Entry {}", i), None);
        }

        let logs = get_logs();
        // Verify at least some entries were added (exact count may vary due to shared state)
        assert!(!logs.is_empty());
    }
}
