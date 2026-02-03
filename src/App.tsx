import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Viewer3D from "./components/Viewer3D";
import "./App.css";

interface FileMetadata {
  path: string;
  fileName: string;
  dimensions?: {
    i: number;
    j: number;
    k: number;
  };
  numberOfGrids?: number;
}

function App() {
  const [gridData, setGridData] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [fileMetadata, setFileMetadata] = useState<FileMetadata | null>(null);

  async function loadFile() {
    try {
      setLoading(true);
      setError("");

      // Open file dialog
      const filePath = await invoke<string | null>("open_file_dialog");

      if (!filePath) {
        setLoading(false);
        return; // User cancelled
      }

      // Load the PLOT3D file
      const data = await invoke("load_plot3d_file", { path: filePath });
      setGridData(data);

      // Extract metadata
      const fileName = filePath.split(/[/\\]/).pop() || filePath;
      const metadata: FileMetadata = {
        path: filePath,
        fileName: fileName,
      };

      // Add dimensions if available
      if (Array.isArray(data) && data.length > 0) {
        metadata.numberOfGrids = data.length;
        const firstGrid = data[0];
        if (firstGrid && firstGrid.dimensions) {
          metadata.dimensions = firstGrid.dimensions;
        }
      }

      setFileMetadata(metadata);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function loadMultipleFiles() {
    try {
      setLoading(true);
      setError("");

      // Open file dialog for multiple files
      const filePaths = await invoke<string[]>("open_multiple_files_dialog");

      if (!filePaths || filePaths.length === 0) {
        setLoading(false);
        return; // User cancelled or no files selected
      }

      // For now, just load the first file
      // TODO: Handle multiple files properly
      const data = await invoke("load_plot3d_file", { path: filePaths[0] });
      setGridData(data);

      const fileName = filePaths[0].split(/[/\\]/).pop() || filePaths[0];
      const metadata: FileMetadata = {
        path: filePaths[0],
        fileName: fileName,
      };

      if (Array.isArray(data) && data.length > 0) {
        metadata.numberOfGrids = data.length;
        const firstGrid = data[0];
        if (firstGrid && firstGrid.dimensions) {
          metadata.dimensions = firstGrid.dimensions;
        }
      }

      setFileMetadata(metadata);
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
        gap: '20px',
        flexWrap: 'wrap'
      }}>
        <h1 style={{ margin: 0, fontSize: '20px' }}>Mehu - PLOT3D Viewer</h1>
        <div style={{ display: 'flex', gap: '10px' }}>
          <button
            onClick={loadFile}
            disabled={loading}
            style={{
              padding: '8px 16px',
              cursor: loading ? 'not-allowed' : 'pointer',
              background: '#3b82f6',
              border: 'none',
              borderRadius: '4px',
              color: 'white',
              opacity: loading ? 0.7 : 1
            }}
          >
            {loading ? 'Loading...' : 'Open File'}
          </button>
          <button
            onClick={loadMultipleFiles}
            disabled={loading}
            style={{
              padding: '8px 16px',
              cursor: loading ? 'not-allowed' : 'pointer',
              background: '#8b5cf6',
              border: 'none',
              borderRadius: '4px',
              color: 'white',
              opacity: loading ? 0.7 : 1
            }}
          >
            Open Multiple Files
          </button>
        </div>
        {error && <span style={{ color: '#ef4444', fontSize: '14px' }}>{error}</span>}
        {fileMetadata && (
          <div style={{
            marginLeft: 'auto',
            fontSize: '14px',
            display: 'flex',
            flexDirection: 'column',
            gap: '4px'
          }}>
            <div><strong>File:</strong> {fileMetadata.fileName}</div>
            {fileMetadata.numberOfGrids !== undefined && (
              <div><strong>Grids:</strong> {fileMetadata.numberOfGrids}</div>
            )}
            {fileMetadata.dimensions && (
              <div>
                <strong>Dimensions:</strong> {fileMetadata.dimensions.i} × {fileMetadata.dimensions.j} × {fileMetadata.dimensions.k}
              </div>
            )}
          </div>
        )}
      </header>

      <main style={{ flex: 1, position: 'relative' }}>
        <Viewer3D gridData={gridData} />
      </main>
    </div>
  );
}

export default App;
