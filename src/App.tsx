import { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Viewer3D from "./components/Viewer3D";
import { LogViewer } from "./components/LogViewer";
import { logger } from "./utils/logger";
import { groupGridsByFile } from "./utils/gridUtils";
import type { Plot3DGrid } from "./types/plot3d";
import type { GridItem } from "./types/grids";
import "./App.css";

interface FileMetadata {
  fileNames: string[];
  gridCount: number;
}

const GRID_COLORS = [
  "#6366f1",
  "#22c55e",
  "#f97316",
  "#14b8a6",
  "#e11d48",
  "#f59e0b",
  "#0ea5e9",
  "#a855f7",
  "#84cc16",
  "#ef4444",
];

const buildGridItems = (
  grids: Plot3DGrid[],
  filePath: string,
  fileName: string,
  colorOffset: number
): GridItem[] =>
  grids.map((grid, index) => ({
    id: `${filePath}::${index}`,
    grid,
    filePath,
    fileName,
    gridIndex: index,
    color: GRID_COLORS[(index + colorOffset) % GRID_COLORS.length],
    visible: true,
  }));

function App() {
  const [grids, setGrids] = useState<GridItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [fileMetadata, setFileMetadata] = useState<FileMetadata | null>(null);
  const [showLogs, setShowLogs] = useState(false);
  const [selectedGridId, setSelectedGridId] = useState<string | null>(null);
  const [isolateSelected, setIsolateSelected] = useState(false);

  const gridTree = useMemo(() => groupGridsByFile(grids), [grids]);
  const selectedGrid = useMemo(
    () => grids.find((grid) => grid.id === selectedGridId) || null,
    [grids, selectedGridId]
  );

  async function loadFile() {
    try {
      setLoading(true);
      setError("");
      logger.info("Opening file dialog...");

      // Open file dialog
      const filePath = await invoke<string | null>("open_file_dialog");

      if (!filePath) {
        setLoading(false);
        logger.debug("File dialog cancelled");
        return; // User cancelled
      }

      logger.info(`Loading file: ${filePath}`);

      // Load the PLOT3D file
      const data = await invoke<Plot3DGrid[]>("load_plot3d_file", { path: filePath });
      const fileName = filePath.split(/[/\\]/).pop() || filePath;

      const gridItems = buildGridItems(data, filePath, fileName, 0);
      setGrids(gridItems);
      setSelectedGridId(gridItems[0]?.id ?? null);
      setIsolateSelected(false);
      logger.info(`Successfully loaded ${gridItems.length} grid(s)`);

      // Extract metadata
      const metadata: FileMetadata = {
        fileNames: [fileName],
        gridCount: gridItems.length,
      };

      setFileMetadata(metadata);
      logger.info(`File metadata: ${metadata.gridCount} grid(s)`);
    } catch (e) {
      const errorMsg = String(e);
      setError(errorMsg);
      logger.error(errorMsg);
    } finally {
      setLoading(false);
    }
  }

  async function loadMultipleFiles() {
    try {
      setLoading(true);
      setError("");
      logger.info("Opening multiple files dialog...");

      // Open file dialog for multiple files
      const filePaths = await invoke<string[]>("open_multiple_files_dialog");

      if (!filePaths || filePaths.length === 0) {
        setLoading(false);
        logger.debug("Multiple files dialog cancelled");
        return; // User cancelled or no files selected
      }

      logger.info(`Loading ${filePaths.length} file(s)...`);

      const allGrids: GridItem[] = [];
      let colorOffset = 0;

      for (const path of filePaths) {
        const data = await invoke<Plot3DGrid[]>("load_plot3d_file", { path });
        const fileName = path.split(/[/\\]/).pop() || path;

        const gridItems = buildGridItems(data, path, fileName, colorOffset);
        allGrids.push(...gridItems);
        colorOffset += gridItems.length;
      }

      setGrids(allGrids);
      setSelectedGridId(allGrids[0]?.id ?? null);
      setIsolateSelected(false);
      logger.info(`Successfully loaded ${allGrids.length} grid(s) from ${filePaths.length} file(s)`);

      const metadata: FileMetadata = {
        fileNames: filePaths.map((path) => path.split(/[/\\]/).pop() || path),
        gridCount: allGrids.length,
      };

      setFileMetadata(metadata);
      logger.info(`File metadata: ${metadata.gridCount} grid(s)`);
    } catch (e) {
      const errorMsg = String(e);
      setError(errorMsg);
      logger.error(errorMsg);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100vh', overflow: 'hidden' }}>
      <header style={{
        background: '#1e293b',
        color: 'white',
        padding: '10px 20px',
        display: 'flex',
        alignItems: 'center',
        gap: '20px',
        flexWrap: 'wrap',
        flexShrink: 0
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
          }}>
            <div>
              <strong>Files:</strong>{' '}
              {fileMetadata.fileNames.length === 1
                ? fileMetadata.fileNames[0]
                : `${fileMetadata.fileNames[0]} +${fileMetadata.fileNames.length - 1}`}
            </div>
            <div><strong>Grids:</strong> {fileMetadata.gridCount}</div>
          </div>
        )}
      </header>

      <main style={{ flex: 1, position: 'relative', display: 'flex', flexDirection: 'column', overflow: 'hidden', minHeight: 0 }}>
        <div style={{ flex: 1, position: 'relative', overflow: 'hidden', display: 'flex', minHeight: 0 }}>
          <aside
            style={{
              width: '280px',
              background: '#0f172a',
              color: '#e2e8f0',
              borderRight: '1px solid #1f2937',
              display: 'flex',
              flexDirection: 'column',
              padding: '12px',
              gap: '12px',
              overflow: 'auto'
            }}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
              <strong style={{ fontSize: '14px', textTransform: 'uppercase', letterSpacing: '0.08em' }}>Grids</strong>
              <div style={{ fontSize: '12px', color: '#94a3b8' }}>
                {fileMetadata ? `${fileMetadata.gridCount} grid(s) loaded` : 'No grids loaded'}
              </div>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
              <label style={{ display: 'flex', alignItems: 'center', gap: '8px', fontSize: '13px' }}>
                <input
                  type="checkbox"
                  checked={isolateSelected}
                  onChange={(e) => setIsolateSelected(e.target.checked)}
                  disabled={!selectedGridId}
                />
                Isolate selected
              </label>
              <button
                onClick={() => {
                  setGrids((prev) => prev.map((grid) => ({ ...grid, visible: true })));
                  setIsolateSelected(false);
                }}
                style={{
                  padding: '6px 10px',
                  fontSize: '12px',
                  background: '#1d4ed8',
                  border: 'none',
                  color: 'white',
                  borderRadius: '6px'
                }}
              >
                Show all grids
              </button>
            </div>

            {gridTree.length === 0 ? (
              <div style={{ fontSize: '12px', color: '#94a3b8' }}>Load a PLOT3D file to view grids.</div>
            ) : (
              gridTree.map((group) => {
                const allVisible = group.grids.every((grid) => grid.visible);
                return (
                  <details key={group.filePath} open style={{ background: '#111827', borderRadius: '8px', padding: '8px' }}>
                    <summary style={{ cursor: 'pointer', listStyle: 'none' }}>
                      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '8px' }}>
                        <div style={{ display: 'flex', flexDirection: 'column', gap: '2px' }}>
                          <span style={{ fontSize: '13px', fontWeight: 600 }}>{group.fileName}</span>
                          <span style={{ fontSize: '11px', color: '#94a3b8' }}>{group.grids.length} grid(s)</span>
                        </div>
                        <label style={{ display: 'flex', alignItems: 'center', gap: '6px', fontSize: '11px', color: '#cbd5f5' }}>
                          <input
                            type="checkbox"
                            checked={allVisible}
                            onChange={(e) => {
                              const checked = e.target.checked;
                              setGrids((prev) =>
                                prev.map((grid) =>
                                  grid.filePath === group.filePath
                                    ? { ...grid, visible: checked }
                                    : grid
                                )
                              );
                            }}
                          />
                          All
                        </label>
                      </div>
                    </summary>
                    <div style={{ marginTop: '8px', display: 'flex', flexDirection: 'column', gap: '6px' }}>
                      {group.grids.map((grid) => {
                        const isSelected = grid.id === selectedGridId;
                        return (
                          <div
                            key={grid.id}
                            style={{
                              display: 'flex',
                              alignItems: 'center',
                              gap: '8px',
                              padding: '6px',
                              borderRadius: '6px',
                              background: isSelected ? 'rgba(148, 163, 184, 0.2)' : 'transparent',
                            }}
                          >
                            <input
                              type="checkbox"
                              checked={grid.visible}
                              onChange={(e) => {
                                const checked = e.target.checked;
                                setGrids((prev) =>
                                  prev.map((item) =>
                                    item.id === grid.id
                                      ? { ...item, visible: checked }
                                      : item
                                  )
                                );
                              }}
                            />
                            <button
                              onClick={() => setSelectedGridId(grid.id)}
                              style={{
                                flex: 1,
                                background: 'transparent',
                                border: 'none',
                                color: '#e2e8f0',
                                textAlign: 'left',
                                padding: 0,
                                cursor: 'pointer',
                                fontSize: '12px'
                              }}
                            >
                              <span style={{ display: 'inline-flex', alignItems: 'center', gap: '8px' }}>
                                <span
                                  style={{
                                    width: '10px',
                                    height: '10px',
                                    borderRadius: '999px',
                                    background: grid.color,
                                    boxShadow: '0 0 0 1px rgba(15, 23, 42, 0.6)'
                                  }}
                                />
                                Grid {grid.gridIndex + 1}
                              </span>
                            </button>
                          </div>
                        );
                      })}
                    </div>
                  </details>
                );
              })
            )}

            {selectedGrid && (
              <div style={{ marginTop: 'auto', background: '#0b1120', padding: '10px', borderRadius: '8px', fontSize: '12px' }}>
                <div style={{ fontWeight: 600, marginBottom: '6px' }}>Selected grid</div>
                <div style={{ color: '#cbd5f5' }}>File: {selectedGrid.fileName}</div>
                <div style={{ color: '#cbd5f5' }}>Grid: {selectedGrid.gridIndex + 1}</div>
                <div style={{ color: '#cbd5f5' }}>
                  Dimensions: {selectedGrid.grid.dimensions.i}x{selectedGrid.grid.dimensions.j}x{selectedGrid.grid.dimensions.k}
                </div>
              </div>
            )}
          </aside>

          <div style={{ flex: 1, position: 'relative', overflow: 'hidden' }}>
            <Viewer3D grids={grids} selectedGridId={selectedGridId} isolateSelected={isolateSelected} />
          </div>
        </div>
        <LogViewer isOpen={showLogs} onToggle={setShowLogs} />
      </main>
    </div>
  );
}

export default App;
