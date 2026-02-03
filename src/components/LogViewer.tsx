import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { logger, LogEntry } from "../utils/logger";
import "./LogViewer.css";

interface LogViewerProps {
    isOpen?: boolean;
    onClose?: () => void;
}

export const LogViewer: React.FC<LogViewerProps> = ({
    isOpen = true,
    onClose,
}) => {
    const [logs, setLogs] = useState<LogEntry[]>([]);
    const [filter, setFilter] = useState<string>("");
    const [levelFilter, setLevelFilter] = useState<string>("ALL");
    const [autoScroll, setAutoScroll] = useState(true);
    const logsEndRef = React.useRef<HTMLDivElement>(null);

    useEffect(() => {
        // Subscribe to log updates
        const unsubscribe = logger.subscribe((newLogs) => {
            setLogs(newLogs);
        });

        // Get initial logs
        setLogs(logger.getLogs());

        return unsubscribe;
    }, []);

    useEffect(() => {
        // Auto-scroll to bottom when new logs appear
        if (autoScroll && logsEndRef.current) {
            logsEndRef.current.scrollIntoView({ behavior: "smooth" });
        }
    }, [logs, autoScroll]);

    const handleClear = async () => {
        if (confirm("Clear all logs?")) {
            logger.clearLogs();
            await logger.clearBackendLogs();
        }
    };

    const handleFetchBackendLogs = async () => {
        const backendLogs = await logger.fetchBackendLogs();
        logger.info(`Fetched ${backendLogs.length} logs from backend`);
    };

  const handleExportLogs = async () => {
    try {
      // Generate filename with timestamp
      const now = new Date();
      const timestamp = now.toISOString().replace(/[:.]/g, "-").split("T")[0];
      const filename = `mehu-logs-${timestamp}.txt`;

      // Use the Downloads directory or current directory
      const downloadsPath = await getDownloadsPath();
      const filePath = `${downloadsPath}/${filename}`;

      logger.info(`Exporting logs to ${filePath}...`, "LogViewer");
      await invoke("export_logs_to_file", { path: filePath });
      logger.info(`Logs successfully exported to ${filename}`, "LogViewer");
      alert(`Logs exported to:\n${filePath}`);
    } catch (error) {
      const errorMsg = `Failed to export logs: ${error}`;
      logger.error(errorMsg, "LogViewer");
      alert(errorMsg);
    }
  };

  const getDownloadsPath = async (): Promise<string> => {
    // Try to use tauri's path resolver if available, otherwise use a default
    try {
      const path = await invoke<string>("get_downloads_path");
      return path;
    } catch {
      // Fallback to home directory or temp
      return process.env.HOME || "/tmp";
    }
  };
    }

    const getLevelColor = (level: string) => {
        switch (level) {
            case "ERROR":
                return "#d32f2f";
            case "WARN":
                return "#f57c00";
            case "INFO":
                return "#388e3c";
            case "DEBUG":
                return "#1976d2";
            default:
                return "#666";
        }
    };

    return (
        <div className="log-viewer">
            <div className="log-header">
                <h3>System Logs</h3>
                <button
                    className="log-close-btn"
                    onClick={onClose}
                    title="Close log viewer"
                >
                    ✕
                </button>
            </div>

            <div className="log-controls">
                <input
                    type="text"
                    placeholder="Filter logs..."
                    value={filter}
                    onChange={(e) => setFilter(e.target.value)}
                    className="log-filter"
                />

                <select
                    value={levelFilter}
                    onChange={(e) => setLevelFilter(e.target.value)}
                    className="log-level-filter"
                >
                    <option value="ALL">All Levels</option>
                    <option value="DEBUG">Debug</option>
                    <option value="INFO">Info</option>
                    <option value="WARN">Warning</option>
                    <option value="ERROR">Error</option>
                </select>

                <label className="log-autoscroll">
                    <input
                        type="checkbox"
                        checked={autoScroll}
                        onChange={(e) => setAutoScroll(e.target.checked)}
                    />
                    Auto-scroll
                </label>

                <button onClick={handleFetchBackendLogs} className="log-action-btn">
                    Fetch Backend
                </button>

                <button onClick={handleExportLogs} className="log-export-btn">
                    Export
                </button>

                <button onClick={handleClear} className="log-clear-btn">
                    Clear
                </button>
            </div>

            <div className="log-entries">
                {filteredLogs.length === 0 ? (
                    <div className="log-empty">
                        {logs.length === 0
                            ? "No logs yet"
                            : "No logs match the current filters"}
                    </div>
                ) : (
                    filteredLogs.map((log, index) => (
                        <div
                            key={index}
                            className={`log-entry log-${log.level.toLowerCase()}`}
                            style={{ borderLeftColor: getLevelColor(log.level) }}
                        >
                            <span className="log-timestamp">{log.timestamp}</span>
                            <span
                                className="log-level"
                                style={{ color: getLevelColor(log.level) }}
                            >
                                {log.level}
                            </span>
                            {log.module && <span className="log-module">{log.module}</span>}
                            <span className="log-message">{log.message}</span>
                        </div>
                    ))
                )}
                <div ref={logsEndRef} />
            </div>

            <div className="log-footer">
                {filteredLogs.length}/{logs.length} entries
            </div>
        </div>
    );
};
