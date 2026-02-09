import type { GridFileGroup, GridItem } from "../types/grids";

export const groupGridsByFile = (grids: GridItem[]): GridFileGroup[] => {
    const map = new Map<string, GridFileGroup>();

    grids.forEach((grid) => {
        if (!map.has(grid.filePath)) {
            map.set(grid.filePath, {
                filePath: grid.filePath,
                fileName: grid.fileName,
                grids: [],
            });
        }
        map.get(grid.filePath)?.grids.push(grid);
    });

    return Array.from(map.values());
};

export const getVisibleGridItems = (
    grids: GridItem[],
    selectedGridIds: string[],
    isolateSelected: boolean
): GridItem[] => {
    const visible = grids.filter((grid) => grid.visible);
    if (!isolateSelected || selectedGridIds.length === 0) {
        return visible;
    }
    return visible.filter((grid) => selectedGridIds.includes(grid.id));
};
