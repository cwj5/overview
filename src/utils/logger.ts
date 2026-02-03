import { invoke } from "@tauri-apps/api/core";

export type LogLevel = "DEBUG" | "INFO" | "WARN" | "ERROR";

export interface LogEntry {
    timestamp: string;
    level: LogLevel;
    message: string;
    module?: string;
    source: "frontend" | "backend";
}

/**
 * Frontend logger that sends logs to the backend
 */
class Logger {
    private logs: LogEntry[] = [];
    private listeners: Set<(logs: LogEntry[]) => void> = new Set();

    /**
     * Log an info message
     */
    info(message: string, module?: string) {
        this.addLog("INFO", message, module);
    }

    /**
     * Log a warning message
     */
    warn(message: string, module?: string) {
        this.addLog("WARN", message, module);
    }

    /**
     * Log an error message
     */
    error(message: string, module?: string) {
        this.addLog("ERROR", message, module);
    }

    /**
     * Log a debug message
     */
    debug(message: string, module?: string) {
        this.addLog("DEBUG", message, module);
    }

    /**
     * Add a log entry and notify listeners
     */
    private addLog(level: LogLevel, message: string, module?: string) {
        const now = new Date();
        const month = String(now.getMonth() + 1).padStart(2, "0");
        const day = String(now.getDate()).padStart(2, "0");
        const hour = String(now.getHours()).padStart(2, "0");
        const minute = String(now.getMinutes()).padStart(2, "0");
        const second = String(now.getSeconds()).padStart(2, "0");
        const timestamp = `${month}-${day} | ${hour}:${minute}:${second}`;

        const entry: LogEntry = {
            timestamp,
            level,
            message,
            module,
            source: "frontend",
        };

        this.logs.push(entry);

        // Keep only last 1000 entries
        if (this.logs.length > 1000) {
            this.logs = this.logs.slice(-1000);
        }

        // Notify listeners
        this.notify();

        // Also log to console
        console[level.toLowerCase() as "debug" | "info" | "warn" | "error"](
            `[${level}] ${module ? `[${module}] ` : ""}${message}`
        );
    }

    /**
     * Get all log entries
     */
    getLogs(): LogEntry[] {
        return [...this.logs];
    }

    /**
     * Fetch logs from backend
     */
    async fetchBackendLogs(): Promise<LogEntry[]> {
        try {
            const backendLogs = await invoke<LogEntry[]>("get_log_entries");
            return backendLogs || [];
        } catch (error) {
            this.error(`Failed to fetch logs from backend: ${error}`);
            return [];
        }
    }

    /**
     * Clear all frontend logs
     */
    clearLogs() {
        this.logs = [];
        this.notify();
    }

    /**
     * Clear backend logs
     */
    async clearBackendLogs(): Promise<void> {
        try {
            await invoke("clear_log_entries");
            this.info("Backend logs cleared");
        } catch (error) {
            this.error(`Failed to clear backend logs: ${error}`);
        }
    }

    /**
     * Subscribe to log updates
     */
    subscribe(listener: (logs: LogEntry[]) => void): () => void {
        this.listeners.add(listener);
        return () => this.listeners.delete(listener);
    }

    /**
     * Notify all listeners of log updates
     */
    private notify() {
        this.listeners.forEach((listener) => listener([...this.logs]));
    }
}

// Export singleton instance
export const logger = new Logger();
