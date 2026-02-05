import { useState, useEffect } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";

export function AboutPage() {
    const [version, setVersion] = useState<string>("0.1.0");

    useEffect(() => {
        getVersion().then(setVersion).catch(() => {
            // Fallback to default version if API fails
            setVersion("0.1.0");
        });
    }, []);

    const openGitHub = async () => {
        console.log("Opening GitHub...");
        try {
            await openUrl("https://github.com/cwj5/mehu");
            console.log("GitHub opened successfully");
        } catch (err) {
            console.error("Failed to open GitHub:", err);
            alert(`Failed to open GitHub: ${err}`);
        }
    };

    return (
        <div
            style={{
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                justifyContent: 'center',
                width: '100%',
                height: '100vh',
                background: '#0f172a',
                color: '#e2e8f0',
                padding: '0',
                fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, sans-serif',
                overflow: 'hidden',
            }}
        >
            <div
                style={{
                    display: 'flex',
                    flexDirection: 'column',
                    alignItems: 'center',
                    justifyContent: 'center',
                    maxWidth: '500px',
                    textAlign: 'center',
                    height: '100%',
                    padding: '20px',
                    boxSizing: 'border-box',
                }}
            >
                <h1 style={{ margin: '0 0 8px 0', fontSize: '32px' }}>Mehu</h1>
                <p style={{ margin: '0 0 4px 0', fontSize: '16px', color: '#94a3b8' }}>
                    PLOT3D Visualization Tool
                </p>
                <p style={{ margin: '0 0 24px 0', fontSize: '16px', color: '#94a3b8' }}>
                    Version {version}
                </p>

                <div
                    style={{
                        background: '#1e293b',
                        padding: '20px',
                        borderRadius: '8px',
                        marginBottom: '20px',
                        textAlign: 'center',
                        fontSize: '13px',
                        lineHeight: '1.7',
                        color: '#cbd5e1',
                        border: '1px solid #334155',
                    }}
                >
                    <p style={{ margin: '0 0 12px 0' }}>
                        <strong style={{ fontSize: '14px' }}>Copyright © 2026 Charles W Jackson</strong>
                    </p>
                    <p style={{ margin: '0 0 10px 0', color: '#94a3b8', fontSize: '12px' }}>
                        Licensed under the Apache License, Version 2.0
                    </p>
                </div>

                <div
                    style={{
                        background: '#1e293b',
                        padding: '16px',
                        borderRadius: '8px',
                        marginBottom: '24px',
                        fontSize: '13px',
                        color: '#cbd5e1',
                        border: '1px solid #334155',
                    }}
                >
                    <p style={{ margin: '0' }}>
                        <strong>"Mehu"</strong> means <strong>"I see"</strong><br></br>
                        (in Twi, approximately)
                    </p>
                </div>

                <button
                    onClick={openGitHub}
                    style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        width: '40px',
                        height: '40px',
                        background: 'transparent',
                        border: 'none',
                        cursor: 'pointer',
                        borderRadius: '6px',
                        transition: 'background-color 0.2s',
                    }}
                    onMouseEnter={(e) => e.currentTarget.style.backgroundColor = '#1e293b'}
                    onMouseLeave={(e) => e.currentTarget.style.backgroundColor = 'transparent'}
                    title="View on GitHub"
                >
                    <svg
                        width="24"
                        height="24"
                        viewBox="0 0 24 24"
                        fill="currentColor"
                        style={{ color: '#e2e8f0' }}
                    >
                        <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v 3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                    </svg>
                </button>
            </div>
        </div>
    );
}
