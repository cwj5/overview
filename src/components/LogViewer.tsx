import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { logger, LogEntry } from "../utils/logger";
import "./LogViewer.css";

interface LogViewerProps {
    isOpen?: boolean;
    onToggle?: (open: boolean) => void;
}

const parseLogTimestamp = (timestamp: string): number => {
    const parsed = Date.parse(timestamp);
    if (!Number.isNaN(parsed)) {
        return parsed;
    }

    // Match format with milliseconds: MM-DD | HH:MM:SS.mmm
    const matchWithMs = timestamp.match(/^(\d{2})-(\d{2})\s*\|\s*(\d{2}):(\d{2}):(\d{2})\.(\d{3})$/);
    if (matchWithMs) {
        const [, month, day, hour, minute, second, millisecond] = matchWithMs;
        const year = new Date().getFullYear();
        return new Date(
            year,
            Number(month) - 1,
            Number(day),
            Number(hour),
            Number(minute),
            Number(second),
            Number(millisecond)
        ).getTime();
    }

    // Fallback: Match format without milliseconds: MM-DD | HH:MM:SS
    const match = timestamp.match(/^(\d{2})-(\d{2})\s*\|\s*(\d{2}):(\d{2}):(\d{2})$/);
    if (match) {
        const [, month, day, hour, minute, second] = match;
        const year = new Date().getFullYear();
        return new Date(
            year,
            Number(month) - 1,
            Number(day),
            Number(hour),
            Number(minute),
            Number(second)
        ).getTime();
    }

    return 0;
};

export const LogViewer: React.FC<LogViewerProps> = ({
    isOpen = false,
    onToggle,
}) => {
    const [logs, setLogs] = useState<LogEntry[]>([]);
    const [filter, setFilter] = useState<string>("");
    const [levelFilter, setLevelFilter] = useState<string>("ALL");
    const [autoScroll, setAutoScroll] = useState(true);
    const [isDrawerOpen, setIsDrawerOpen] = useState(isOpen);
    const logsEndRef = React.useRef<HTMLDivElement>(null);

    useEffect(() => {
        setIsDrawerOpen(isOpen);
    }, [isOpen]);

    useEffect(() => {
        // Subscribe to log updates
        const unsubscribe = logger.subscribe((newLogs) => {
            setLogs(newLogs);
        });

        // Get initial logs
        const loadInitialLogs = async () => {
            const backendLogs = await logger.fetchBackendLogs();
            // Merge backend logs with frontend logs
            const mergedLogs = [...backendLogs, ...logger.getLogs()].sort((a, b) => {
                return parseLogTimestamp(a.timestamp) - parseLogTimestamp(b.timestamp);
            });
            setLogs(mergedLogs);
        };

        loadInitialLogs();

        // Poll for backend logs every 500ms
        const interval = setInterval(async () => {
            const backendLogs = await logger.fetchBackendLogs();
            const mergedLogs = [...backendLogs, ...logger.getLogs()].sort((a, b) => {
                return parseLogTimestamp(a.timestamp) - parseLogTimestamp(b.timestamp);
            });
            setLogs(mergedLogs);
        }, 500);

        return () => {
            clearInterval(interval);
            unsubscribe();
        };
    }, []);

    useEffect(() => {
        // Auto-scroll to bottom when new logs appear
        if (autoScroll && logsEndRef.current) {
            logsEndRef.current.scrollIntoView({ behavior: "smooth" });
        }
    }, [logs, autoScroll]);

    const handleToggle = () => {
        const newState = !isDrawerOpen;
        setIsDrawerOpen(newState);
        onToggle?.(newState);
    };

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
            logger.info("Opening save dialog for log export...");

            // Open save file dialog
            const filePath = await invoke<string | null>("save_log_file_dialog");

            if (!filePath) {
                logger.debug("Log export cancelled");
                return; // User cancelled
            }

            logger.info(`Exporting logs to ${filePath}...`);

            // Get all logs (both frontend and backend merged)
            const backendLogs = await logger.fetchBackendLogs();
            const allLogs = [...backendLogs, ...logger.getLogs()].sort((a, b) => {
                return parseLogTimestamp(a.timestamp) - parseLogTimestamp(b.timestamp);
            });

            // Format logs as text
            let logText = "Mehu PLOT3D Viewer - Log Export\n";
            logText += "================================\n";
            logText += `Exported: ${new Date().toLocaleString()}\n`;
            logText += `Total entries: ${allLogs.length}\n`;
            logText += "================================\n\n";

            for (const log of allLogs) {
                const moduleStr = log.module ? ` [${log.module}]` : '';
                logText += `[${log.timestamp}] ${log.source} ${log.level}${moduleStr} ${log.message}\n`;
            }

            // Write to file using Tauri's fs
            await invoke("write_text_file", { path: filePath, contents: logText });

            logger.info(`Logs successfully exported (${allLogs.length} entries)`);
            alert(`Logs exported to:\n${filePath}\n\n${allLogs.length} entries written`);
        } catch (error) {
            const errorMsg = `Failed to export logs: ${error}`;
            logger.error(errorMsg);
            alert(errorMsg);
        }
    };

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

    const filteredLogs = logs.filter((log) => {
        const matchesText = log.message
            .toLowerCase()
            .includes(filter.toLowerCase());
        const matchesLevel = levelFilter === "ALL" || log.level === levelFilter;
        return matchesText && matchesLevel;
    });

    return (
        <>
            {/* Drawer container */}
            <div className={`log-drawer ${isDrawerOpen ? "open" : "closed"}`}>
                <div className="log-viewer">
                    <div className="log-header">
                        <h3>System Logs</h3>
                        <button
                            className="log-close-btn"
                            onClick={handleToggle}
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
                                        {log.level === "DEBUG" ? "🐛" : log.level === "INFO" ? "ℹ️" : log.level === "WARN" ? "⚠️" : "❌"}
                                    </span>
                                    <span className="log-source" style={{ color: log.source === "🦀" || log.source === "backend" ? "#9c27b0" : "#4a90e2" }}>
                                        {log.source}
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
            </div>

            {/* Tab button at bottom - only show when drawer is closed */}
            {!isDrawerOpen && (
                <button
                    className="log-tab"
                    onClick={handleToggle}
                    title="Open logs"
                >
                    <span className="log-tab-icon">📋</span>
                    <span className="log-tab-text">Logs</span>
                    <span className="log-tab-badge">{logs.length}</span>
                </button>
            )}
        </>
    );
};
