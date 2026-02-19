/**
 * Unit tests for logger utilities
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { logger, parseLogTimestamp } from './logger';

describe('logger', () => {
    beforeEach(() => {
        // Clear logs before each test
        const logs = logger.getLogs();
        logs.forEach(() => {
            // Logs accumulate, but we can check their length
        });
    });

    describe('parseLogTimestamp', () => {
        it('should parse ISO date format', () => {
            const timestamp = '2026-02-18T12:30:45.000Z';
            const result = parseLogTimestamp(timestamp);
            expect(result).toBeGreaterThan(0);
        });

        it('should parse custom format with milliseconds', () => {
            const timestamp = '02-18 | 12:30:45.123';
            const result = parseLogTimestamp(timestamp);
            expect(result).toBeGreaterThan(0);
        });

        it('should parse custom format without milliseconds', () => {
            const timestamp = '02-18 | 12:30:45';
            const result = parseLogTimestamp(timestamp);
            expect(result).toBeGreaterThan(0);
        });

        it('should return 0 for invalid format', () => {
            const timestamp = 'invalid-timestamp';
            const result = parseLogTimestamp(timestamp);
            expect(result).toBe(0);
        });

        it('should parse month and day correctly', () => {
            const timestamp1 = '01-01 | 00:00:00';
            const timestamp2 = '12-31 | 23:59:59';

            const result1 = parseLogTimestamp(timestamp1);
            const result2 = parseLogTimestamp(timestamp2);

            expect(result1).toBeGreaterThan(0);
            expect(result2).toBeGreaterThan(0);
            expect(result2).toBeGreaterThan(result1);
        });
    });

    describe('Logger class', () => {
        describe('info', () => {
            it('should log info messages', () => {
                logger.info('Test info message');
                const logs = logger.getLogs();

                expect(logs.length).toBeGreaterThan(0);
                const lastLog = logs[logs.length - 1];
                expect(lastLog.level).toBe('INFO');
                expect(lastLog.message).toBe('Test info message');
            });

            it('should include module name', () => {
                logger.info('Test message', 'TestModule');
                const logs = logger.getLogs();
                const lastLog = logs[logs.length - 1];

                expect(lastLog.module).toBe('TestModule');
            });

            it('should set source to ⚛️', () => {
                logger.info('Test message');
                const logs = logger.getLogs();
                const lastLog = logs[logs.length - 1];

                expect(lastLog.source).toBe('⚛️');
            });
        });

        describe('warn', () => {
            it('should log warning messages', () => {
                logger.warn('Test warning');
                const logs = logger.getLogs();
                const lastLog = logs[logs.length - 1];

                expect(lastLog.level).toBe('WARN');
                expect(lastLog.message).toBe('Test warning');
            });
        });

        describe('error', () => {
            it('should log error messages', () => {
                logger.error('Test error');
                const logs = logger.getLogs();
                const lastLog = logs[logs.length - 1];

                expect(lastLog.level).toBe('ERROR');
                expect(lastLog.message).toBe('Test error');
            });
        });

        describe('debug', () => {
            it('should log debug messages', () => {
                logger.debug('Test debug');
                const logs = logger.getLogs();
                const lastLog = logs[logs.length - 1];

                expect(lastLog.level).toBe('DEBUG');
                expect(lastLog.message).toBe('Test debug');
            });
        });

        describe('getLogs', () => {
            it('should return array of logs', () => {
                logger.debug('Test 1');
                logger.info('Test 2');
                const logs = logger.getLogs();

                expect(Array.isArray(logs)).toBe(true);
                expect(logs.length).toBeGreaterThanOrEqual(2);
            });

            it('should maintain log order', () => {
                logger.debug('First');
                logger.info('Second');
                logger.warn('Third');
                const logs = logger.getLogs();

                const lastThree = logs.slice(-3);
                expect(lastThree[0].message).toBe('First');
                expect(lastThree[1].message).toBe('Second');
                expect(lastThree[2].message).toBe('Third');
            });
        });

        describe('subscribe', () => {
            it('should notify subscribers on new logs', () => {
                let subscribeCallCount = 0;
                const unsubscribe = logger.subscribe(() => {
                    subscribeCallCount++;
                });

                logger.info('Test message');

                expect(subscribeCallCount).toBeGreaterThan(0);
                unsubscribe();
            });

            it('should return unsubscribe function', () => {
                const unsubscribe = logger.subscribe(() => { });
                expect(typeof unsubscribe).toBe('function');
                unsubscribe();
            });

            it('should not notify after unsubscribe', () => {
                let callCount = 0;
                const unsubscribe = logger.subscribe(() => {
                    callCount++;
                });

                logger.info('Message 1');
                const count1 = callCount;

                unsubscribe();
                logger.info('Message 2');
                const count2 = callCount;

                expect(count2).toBe(count1);
            });
        });

        describe('timestamp format', () => {
            it('should include valid timestamp in log entry', () => {
                logger.info('Test message');
                const logs = logger.getLogs();
                const lastLog = logs[logs.length - 1];

                // Check timestamp matches MM-DD | HH:MM:SS.mmm format
                expect(lastLog.timestamp).toMatch(/^\d{2}-\d{2}\s\|\s\d{2}:\d{2}:\d{2}\.\d{3}$/);
            });

            it('should have valid date components', () => {
                logger.info('Test');
                const logs = logger.getLogs();
                const lastLog = logs[logs.length - 1];

                const match = lastLog.timestamp.match(/^(\d{2})-(\d{2})\s\|\s(\d{2}):(\d{2}):(\d{2})\.(\d{3})$/);
                expect(match).toBeTruthy();

                if (match) {
                    const [, month, day, hour, minute, second, ms] = match;

                    expect(parseInt(month)).toBeGreaterThanOrEqual(1);
                    expect(parseInt(month)).toBeLessThanOrEqual(12);
                    expect(parseInt(day)).toBeGreaterThanOrEqual(1);
                    expect(parseInt(day)).toBeLessThanOrEqual(31);
                    expect(parseInt(hour)).toBeGreaterThanOrEqual(0);
                    expect(parseInt(hour)).toBeLessThanOrEqual(23);
                    expect(parseInt(minute)).toBeGreaterThanOrEqual(0);
                    expect(parseInt(minute)).toBeLessThanOrEqual(59);
                    expect(parseInt(second)).toBeGreaterThanOrEqual(0);
                    expect(parseInt(second)).toBeLessThanOrEqual(59);
                    expect(parseInt(ms)).toBeGreaterThanOrEqual(0);
                    expect(parseInt(ms)).toBeLessThanOrEqual(999);
                }
            });
        });

        describe('fetchBackendLogs', () => {
            it('should return array from backend', async () => {
                const logs = await logger.fetchBackendLogs();
                expect(Array.isArray(logs)).toBe(true);
            });
        });

        describe('clearLogs', () => {
            it('should clear all logs', () => {
                logger.info('Test message');
                let logsCount = logger.getLogs().length;
                expect(logsCount).toBeGreaterThan(0);

                logger.clearLogs();
                logsCount = logger.getLogs().length;
                expect(logsCount).toBe(0);
            });

            it('should notify subscribers on clear', () => {
                let subscribeCallCount = 0;
                const unsubscribe = logger.subscribe(() => {
                    subscribeCallCount++;
                });

                logger.clearLogs();

                expect(subscribeCallCount).toBeGreaterThan(0);
                unsubscribe();
            });
        });

        describe('log levels', () => {
            it('should distinguish between log levels', () => {
                const initialCount = logger.getLogs().length;

                logger.debug('Debug');
                logger.info('Info');
                logger.warn('Warn');
                logger.error('Error');

                const logs = logger.getLogs().slice(initialCount);
                const levels = logs.map(l => l.level);

                expect(levels).toContain('DEBUG');
                expect(levels).toContain('INFO');
                expect(levels).toContain('WARN');
                expect(levels).toContain('ERROR');
            });
        });
    });
});
