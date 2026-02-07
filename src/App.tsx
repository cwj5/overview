// Copyright 2026 Charles W Jackson
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import { useMemo, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Menu, MenuItem, Submenu } from "@tauri-apps/api/menu";
import Viewer3D from "./components/Viewer3D";
import { LogViewer } from "./components/LogViewer";
import { SolutionViewer } from "./components/SolutionViewer";
import { logger } from "./utils/logger";
import { groupGridsByFile } from "./utils/gridUtils";
import type { Plot3DGrid, Plot3DSolution } from "./types/plot3d";
import type { GridItem } from "./types/grids";
import type { ScalarField } from "./utils/solutionData";
import type { ColorScheme } from "./utils/colorMapping";
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
  const [hasSolution, setHasSolution] = useState(false);
  const [ignoreIblank, setIgnoreIblank] = useState(false);
  const [currentScalarField, setCurrentScalarField] = useState<ScalarField>('none');
  const [currentColorScheme, setCurrentColorScheme] = useState<ColorScheme>('viridis');

  // Check if any grid has IBLANK data
  const hasIblankData = useMemo(() => {
    return grids.some((grid) => grid.grid.iblank !== null && grid.grid.iblank !== undefined);
  }, [grids]);

  useEffect(() => {
    const setupMenu = async () => {
      try {
        const aboutItem = await MenuItem.new({
          id: "about",
          text: "About Mehu",
          action: () => {
            invoke("open_about_window").catch((err) =>
              logger.error(`Failed to open About window: ${err}`)
            );
          },
        });

        const ignoreIblankItem = await MenuItem.new({
          id: "ignore-iblank",
          text: "Ignore IBLANK",
          enabled: hasIblankData,
          action: () => {
            setIgnoreIblank((prev) => !prev);
          },
        });

        const fileSubmenu = await Submenu.new({
          text: "File",
          items: [aboutItem],
        });

        const viewSubmenu = await Submenu.new({
          text: "View",
          items: [ignoreIblankItem],
        });

        const menu = await Menu.new({
          items: [fileSubmenu, viewSubmenu],
        });

        await menu.setAsAppMenu();
      } catch (err) {
        logger.error(`Failed to setup menu: ${err}`);
      }
    };

    setupMenu();
  }, [hasIblankData]);

  // Reset ignoreIblank when IBLANK data is no longer available
  useEffect(() => {
    if (!hasIblankData && ignoreIblank) {
      setIgnoreIblank(false);
    }
  }, [hasIblankData, ignoreIblank]);

  const gridTree = useMemo(() => groupGridsByFile(grids), [grids]);
  const selectedGrid = useMemo(
    () => grids.find((grid) => grid.id === selectedGridId) || null,
    [grids, selectedGridId]
  );

  async function loadFiles() {
    try {
      setLoading(true);
      setError("");
      logger.info("Opening file selection dialog...");

      // Open file dialog for selecting one or more files
      const filePaths = await invoke<string[]>("open_multiple_files_dialog");

      if (!filePaths || filePaths.length === 0) {
        setLoading(false);
        logger.debug("File dialog cancelled");
        return;
      }

      logger.info(`Loading ${filePaths.length} file(s)...`);

      // Try to load each file as a grid, collect successful grids
      const gridResults: { path: string; grids: Plot3DGrid[]; fileName: string }[] = [];
      const potentialSolutionPaths: string[] = [];

      for (const path of filePaths) {
        try {
          const grids = await invoke<Plot3DGrid[]>("load_plot3d_file", { path });
          const fileName = path.split(/[/\\]/).pop() || path;
          gridResults.push({ path, grids, fileName });
          logger.info(`Loaded ${grids.length} grid(s) from ${fileName}`);
        } catch (e) {
          // If it fails as a grid, it might be a solution file
          potentialSolutionPaths.push(path);
          logger.debug(`${path} is not a grid file, will try as solution`);
        }
      }

      if (gridResults.length === 0) {
        throw new Error("No valid grid files found in selection");
      }

      // Build grid items from all loaded grids
      const allGrids: GridItem[] = [];
      let colorOffset = 0;

      for (const { path, grids, fileName } of gridResults) {
        const gridItems = buildGridItems(grids, path, fileName, colorOffset);
        allGrids.push(...gridItems);
        colorOffset += gridItems.length;
      }

      setGrids(allGrids);
      setSelectedGridId(allGrids[0]?.id ?? null);
      setIsolateSelected(false);
      setHasSolution(false);

      // Try to load solution files
      if (potentialSolutionPaths.length > 0) {
        for (const solPath of potentialSolutionPaths) {
          try {
            // Auto-detect binary or ASCII format and load accordingly
            const solutions = await invoke<Plot3DSolution[]>("load_plot3d_solution_auto", { path: solPath });

            // Validate solution matches grids
            if (solutions.length !== allGrids.length) {
              throw new Error(
                `Solution file has ${solutions.length} grid(s) but grid file has ${allGrids.length} grid(s). They must match.`
              );
            }

            // Validate dimensions for each grid
            for (let i = 0; i < solutions.length; i++) {
              const solution = solutions[i];
              const gridItem = allGrids.find((g) => g.gridIndex === solution.grid_index);

              if (!gridItem) {
                throw new Error(`Solution grid ${solution.grid_index + 1} not found in loaded grids`);
              }

              const grid = gridItem.grid;
              if (
                solution.dimensions.i !== grid.dimensions.i ||
                solution.dimensions.j !== grid.dimensions.j ||
                solution.dimensions.k !== grid.dimensions.k
              ) {
                throw new Error(
                  `Grid ${solution.grid_index + 1} dimensions mismatch: solution has ${solution.dimensions.i}x${solution.dimensions.j}x${solution.dimensions.k} but grid has ${grid.dimensions.i}x${grid.dimensions.j}x${grid.dimensions.k}`
                );
              }
            }

            // Match solutions to grids
            setGrids((prevGrids) =>
              prevGrids.map((gridItem) => {
                const solution = solutions.find((sol) => sol.grid_index === gridItem.gridIndex);
                if (solution) {
                  return { ...gridItem, solution };
                }
                return gridItem;
              })
            );

            setHasSolution(true);
            logger.info(`Successfully loaded ${solutions.length} solution(s) from ${solPath.split(/[/\\]/).pop()}`);
          } catch (e) {
            const errorMsg = String(e).replace(/^Error:\s*/, '');
            logger.error(errorMsg);
            throw new Error(errorMsg);
          }
        }
      }

      const metadata: FileMetadata = {
        fileNames: gridResults.map(r => r.fileName),
        gridCount: allGrids.length,
      };

      setFileMetadata(metadata);
      logger.info(`Loaded ${metadata.gridCount} total grid(s) from ${gridResults.length} file(s)`);
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
        <div style={{ display: 'flex', alignItems: 'center', gap: '20px' }}>
          <h1 style={{ margin: 0, fontSize: '20px' }}>Mehu - PLOT3D Viewer</h1>
        </div>
        <div style={{ display: 'flex', gap: '10px' }}>
          <button
            onClick={loadFiles}
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
            {loading ? 'Loading...' : 'Load Files'}
          </button>
          {hasSolution && (
            <span style={{
              display: 'flex',
              alignItems: 'center',
              gap: '6px',
              padding: '8px 12px',
              background: '#10b981',
              borderRadius: '4px',
              color: 'white',
              fontSize: '13px'
            }}>
              ✓ Solution loaded
            </span>
          )}
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
                                {grid.solution && (
                                  <span style={{ fontSize: '10px', color: '#10b981' }}>●</span>
                                )}
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
                {selectedGrid.solution && (
                  <div style={{ color: '#10b981', marginTop: '4px', fontSize: '11px' }}>
                    ✓ Solution data loaded
                  </div>
                )}
              </div>
            )}

            {selectedGrid && selectedGrid.solution && (
              <div style={{ marginTop: '12px' }}>
                <SolutionViewer
                  selectedGrid={selectedGrid}
                  onScalarFieldChange={setCurrentScalarField}
                  onColorSchemeChange={setCurrentColorScheme}
                />
              </div>
            )}
          </aside>

          <div style={{ flex: 1, position: 'relative', overflow: 'hidden' }}>
            <Viewer3D
              grids={grids}
              selectedGridId={selectedGridId}
              isolateSelected={isolateSelected}
              ignoreIblank={ignoreIblank}
              scalarField={currentScalarField}
              colorScheme={currentColorScheme}
            />
          </div>
        </div>
        <LogViewer isOpen={showLogs} onToggle={setShowLogs} />
      </main>
    </div>
  );
}

export default App;
