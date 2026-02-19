/**
 * Application-wide constants
 */

// Physics constants
export const DEFAULT_GAMMA = 1.4; // Default specific heat ratio for air

// Logging constants
export const MAX_LOG_ENTRIES = 1000;
export const LOG_TIMESTAMP_FORMAT = {
    MS_PATTERN: /^(\d{2})-(\d{2})\s*\|\s*(\d{2}):(\d{2}):(\d{2})\.(\d{3})$/,
    NO_MS_PATTERN: /^(\d{2})-(\d{2})\s*\|\s*(\d{2}):(\d{2}):(\d{2})$/,
} as const;

// Grid visualization constants
export const GRID_COLORS = [
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
] as const;

// Rendering constants
export const MESH_RENDERING = {
    CHUNK_SIZE: 50000,
    DOUBLE_SIDE: 2,
    DEFAULT_OPACITY: 1.0,
    DIMMED_OPACITY: 0.35,
    BACKFACE_MULTIPLIER: 0.3,
    FRONT_AMBIENT: 0.7,
} as const;

export const LIGHT_SOURCES = {
    light1: { x: 0.5, y: 0.5, z: 1.0 },
    light2: { x: -0.5, y: -0.3, z: 0.8, multiplier: 0.5 },
    light3: { x: 0.0, y: 1.0, z: 0.3, multiplier: 0.3 },
} as const;

// Loading state constants
export const LOADING_MESSAGES = {
    DEFAULT: "Processing...",
    DIALOG: "Opening file dialog...",
    PARSING: (fileName: string) => `Parsing ${fileName}...`,
    LOADING: (count: number) => `Loading ${count} file(s)...`,
    SOLUTION: "Loading solution data...",
    COMPUTING_COLORS: "Computing solution colors...",
} as const;

// Color normalization constants
export const COLOR_NORMALIZATION = {
    SAMPLE_COUNT: 3000,
    MAX_8BIT: 255.0,
} as const;

// Number formatting constants
export const NUMBER_FORMAT = {
    DECIMALS: 3,
    SMALL_THRESHOLD: 0.001,
    LARGE_THRESHOLD: 1000,
} as const;

// Animation/performance constants
export const PERFORMANCE = {
    RAF_DELAY: 0,
    TIMEOUT_ZERO: 0,
} as const;
