import { describe, expect, it } from "vitest";
import { groupGridsByFile, getVisibleGridItems } from "./gridUtils";
import type { GridItem } from "../types/grids";
import type { Plot3DGrid } from "../types/plot3d";

const makeGrid = (): Plot3DGrid => ({
    dimensions: { i: 2, j: 2, k: 2 },
    x_coords: [0, 1, 0, 1, 0, 1, 0, 1],
    y_coords: [0, 0, 1, 1, 0, 0, 1, 1],
    z_coords: [0, 0, 0, 0, 1, 1, 1, 1],
});

const makeItem = (overrides: Partial<GridItem>): GridItem => ({
    id: overrides.id ?? "fileA::0",
    grid: overrides.grid ?? makeGrid(),
    filePath: overrides.filePath ?? "/tmp/fileA.p3d",
    fileName: overrides.fileName ?? "fileA.p3d",
    gridIndex: overrides.gridIndex ?? 0,
    color: overrides.color ?? "#6366f1",
    visible: overrides.visible ?? true,
});

describe("groupGridsByFile", () => {
    it("groups grids by file while preserving order", () => {
        const grids: GridItem[] = [
            makeItem({ id: "fileA::0" }),
            makeItem({ id: "fileB::0", filePath: "/tmp/fileB.p3d", fileName: "fileB.p3d" }),
            makeItem({ id: "fileA::1", gridIndex: 1 }),
        ];

        const grouped = groupGridsByFile(grids);

        expect(grouped).toHaveLength(2);
        expect(grouped[0].fileName).toBe("fileA.p3d");
        expect(grouped[0].grids).toHaveLength(2);
        expect(grouped[1].fileName).toBe("fileB.p3d");
        expect(grouped[1].grids).toHaveLength(1);
    });
});

describe("getVisibleGridItems", () => {
    it("returns only visible grids when not isolating", () => {
        const grids: GridItem[] = [
            makeItem({ id: "fileA::0", visible: true }),
            makeItem({ id: "fileA::1", gridIndex: 1, visible: false }),
        ];

        const visible = getVisibleGridItems(grids, null, false);
        expect(visible).toHaveLength(1);
        expect(visible[0].id).toBe("fileA::0");
    });

    it("returns only selected grid when isolating", () => {
        const grids: GridItem[] = [
            makeItem({ id: "fileA::0", visible: true }),
            makeItem({ id: "fileA::1", gridIndex: 1, visible: true }),
        ];

        const visible = getVisibleGridItems(grids, "fileA::1", true);
        expect(visible).toHaveLength(1);
        expect(visible[0].id).toBe("fileA::1");
    });
});
