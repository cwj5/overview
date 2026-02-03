import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Viewer3D from "./components/Viewer3D";
import "./App.css";

function App() {
  const [gridData, setGridData] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function loadFile() {
    try {
      setLoading(true);
      setError("");
      // TODO: Add file dialog to select PLOT3D file
      // For now, this is a placeholder
      const data = await invoke("load_plot3d_file", { path: "/path/to/file.grid" });
      setGridData(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh' }}>
      <header style={{
        background: '#1e293b',
        color: 'white',
        padding: '10px 20px',
        display: 'flex',
        alignItems: 'center',
        gap: '20px'
      }}>
        <h1 style={{ margin: 0, fontSize: '20px' }}>Mehu - PLOT3D Viewer</h1>
        <button
          onClick={loadFile}
          disabled={loading}
          style={{
            padding: '8px 16px',
            cursor: 'pointer',
            background: '#3b82f6',
            border: 'none',
            borderRadius: '4px',
            color: 'white'
          }}
        >
          {loading ? 'Loading...' : 'Load PLOT3D File'}
        </button>
        {error && <span style={{ color: '#ef4444' }}>{error}</span>}
      </header>

      <main style={{ flex: 1, position: 'relative' }}>
        <Viewer3D gridData={gridData} />
      </main>
    </div>
  );
}

export default App;
