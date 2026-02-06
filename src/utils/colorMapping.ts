/**
 * Color mapping utilities for scalar field visualization
 */

export type ColorScheme = 'rainbow' | 'viridis' | 'turbo' | 'grayscale' | 'hot';

/**
 * RGB color as [r, g, b] with values 0-1
 */
export interface RGB {
    r: number;
    g: number;
    b: number;
}

/**
 * Map a normalized value [0, 1] to RGB color based on scheme
 */
export function mapValueToColor(value: number, scheme: ColorScheme = 'viridis'): RGB {
    // Clamp value to [0, 1]
    const v = Math.max(0, Math.min(1, value));

    switch (scheme) {
        case 'rainbow':
            return rainbowColor(v);
        case 'viridis':
            return viridisColor(v);
        case 'turbo':
            return turboColor(v);
        case 'grayscale':
            return { r: v, g: v, b: v };
        case 'hot':
            return hotColor(v);
        default:
            return viridisColor(v);
    }
}

/**
 * Rainbow colormap: red -> yellow -> green -> cyan -> blue -> magenta
 */
function rainbowColor(v: number): RGB {
    let r = 0, g = 0, b = 0;

    if (v < 0.2) {
        // Red to yellow
        r = 1;
        g = v / 0.2;
    } else if (v < 0.4) {
        // Yellow to green
        r = 1 - (v - 0.2) / 0.2;
        g = 1;
    } else if (v < 0.6) {
        // Green to cyan
        g = 1;
        b = (v - 0.4) / 0.2;
    } else if (v < 0.8) {
        // Cyan to blue
        g = 1 - (v - 0.6) / 0.2;
        b = 1;
    } else {
        // Blue to magenta
        r = (v - 0.8) / 0.2;
        b = 1;
    }

    return { r, g, b };
}

/**
 * Viridis colormap (perceptually uniform)
 * Approximation of matplotlib's viridis
 */
function viridisColor(v: number): RGB {
    // Viridis: dark purple -> green -> yellow
    const lut = [
        [0.267004, 0.004874, 0.329415],
        [0.282623, 0.140461, 0.469470],
        [0.253935, 0.265254, 0.529983],
        [0.206756, 0.371758, 0.553806],
        [0.163625, 0.471133, 0.558695],
        [0.127568, 0.566949, 0.550413],
        [0.134692, 0.658636, 0.517649],
        [0.266941, 0.748751, 0.440573],
        [0.477504, 0.821444, 0.318195],
        [0.741388, 0.873449, 0.149561],
        [0.993248, 0.906157, 0.143936],
    ];

    const idx = Math.floor(v * (lut.length - 1));
    const t = (v * (lut.length - 1)) - idx;
    const nextIdx = Math.min(idx + 1, lut.length - 1);

    const c1 = lut[idx];
    const c2 = lut[nextIdx];

    return {
        r: c1[0] * (1 - t) + c2[0] * t,
        g: c1[1] * (1 - t) + c2[1] * t,
        b: c1[2] * (1 - t) + c2[2] * t,
    };
}

/**
 * Turbo colormap (blue -> green -> yellow -> red)
 */
function turboColor(v: number): RGB {
    const lut = [
        [0.18995, 0.07176, 0.23217],
        [0.19483, 0.08339, 0.26149],
        [0.19956, 0.09498, 0.29024],
        [0.20415, 0.10652, 0.31844],
        [0.20860, 0.11802, 0.34607],
        [0.21291, 0.12947, 0.37314],
        [0.21708, 0.14087, 0.39964],
        [0.22112, 0.15223, 0.42558],
        [0.22500, 0.16354, 0.45096],
        [0.22873, 0.17481, 0.47578],
        [0.23231, 0.18603, 0.50004],
        [0.23573, 0.19720, 0.52373],
        [0.23900, 0.20833, 0.54686],
        [0.24218, 0.21941, 0.56942],
        [0.24519, 0.23044, 0.59142],
        [0.24812, 0.24143, 0.61286],
        [0.25093, 0.25237, 0.63374],
        [0.25362, 0.26327, 0.65406],
        [0.25621, 0.27412, 0.67381],
        [0.25868, 0.28492, 0.69300],
        [0.26109, 0.29568, 0.71162],
        [0.26337, 0.30639, 0.72968],
        [0.26559, 0.31706, 0.74718],
        [0.26763, 0.32768, 0.76412],
        [0.26956, 0.33825, 0.78050],
        [0.27140, 0.34878, 0.79631],
        [0.27314, 0.35926, 0.81156],
        [0.27479, 0.36970, 0.82624],
        [0.27635, 0.38008, 0.84037],
        [0.27778, 0.39042, 0.85393],
        [0.27914, 0.40070, 0.86692],
        [0.28037, 0.41093, 0.87936],
        [0.28153, 0.42110, 0.89123],
        [0.28259, 0.43121, 0.90254],
        [0.28356, 0.44127, 0.91328],
        [0.28445, 0.45125, 0.92347],
        [0.28525, 0.46118, 0.93309],
        [0.28596, 0.47105, 0.94214],
        [0.28658, 0.48087, 0.95064],
        [0.28711, 0.49062, 0.95857],
    ];

    const idx = Math.floor(v * (lut.length - 1));
    const t = (v * (lut.length - 1)) - idx;
    const nextIdx = Math.min(idx + 1, lut.length - 1);

    const c1 = lut[idx];
    const c2 = lut[nextIdx];

    return {
        r: c1[0] * (1 - t) + c2[0] * t,
        g: c1[1] * (1 - t) + c2[1] * t,
        b: c1[2] * (1 - t) + c2[2] * t,
    };
}

/**
 * Hot colormap: black -> red -> yellow -> white
 */
function hotColor(v: number): RGB {
    if (v < 0.33) {
        return {
            r: v / 0.33,
            g: 0,
            b: 0,
        };
    } else if (v < 0.66) {
        return {
            r: 1,
            g: (v - 0.33) / 0.33,
            b: 0,
        };
    } else {
        return {
            r: 1,
            g: 1,
            b: (v - 0.66) / 0.34,
        };
    }
}

/**
 * Convert RGB (0-1) to 8-bit hex string
 */
export function rgbToHex(color: RGB): string {
    const r = Math.round(color.r * 255);
    const g = Math.round(color.g * 255);
    const b = Math.round(color.b * 255);
    return `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`;
}

/**
 * Normalize value to [0, 1] range based on min/max
 */
export function normalizeValue(value: number, min: number, max: number): number {
    if (min === max) return 0.5;
    return (value - min) / (max - min);
}

/**
 * Generate color array for a set of values
 */
export function generateColorArray(
    values: number[],
    scheme: ColorScheme = 'viridis',
    min?: number,
    max?: number
): Float32Array {
    if (values.length === 0) {
        return new Float32Array(0);
    }

    // Calculate min/max if not provided
    const actualMin = min ?? Math.min(...values);
    const actualMax = max ?? Math.max(...values);

    // Generate RGB values (interleaved as r, g, b, r, g, b, ...)
    const colors = new Float32Array(values.length * 3);

    for (let i = 0; i < values.length; i++) {
        const normalized = normalizeValue(values[i], actualMin, actualMax);
        const color = mapValueToColor(normalized, scheme);

        colors[i * 3] = color.r;
        colors[i * 3 + 1] = color.g;
        colors[i * 3 + 2] = color.b;
    }

    return colors;
}
