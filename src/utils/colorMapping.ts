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
 * Turbo colormap (purple -> blue -> cyan -> green -> yellow -> orange -> red)
 * Google's Turbo color scheme for better perceptual uniformity
 */
function turboColor(v: number): RGB {
    // Google Turbo colormap sampled at 16 key points
    const lut = [
        [0.19, 0.07, 0.23],  // dark purple/blue
        [0.21, 0.14, 0.42],  // purple-blue
        [0.24, 0.26, 0.61],  // blue
        [0.27, 0.38, 0.81],  // cyan-blue
        [0.29, 0.50, 0.93],  // cyan
        [0.28, 0.63, 0.94],  // cyan-green
        [0.25, 0.74, 0.80],  // green
        [0.42, 0.84, 0.54],  // yellow-green
        [0.67, 0.90, 0.28],  // yellow
        [0.89, 0.88, 0.12],  // orange-yellow
        [1.00, 0.77, 0.06],  // orange
        [1.00, 0.60, 0.03],  // orange-red
        [0.97, 0.40, 0.02],  // red-orange
        [0.92, 0.20, 0.01],  // red
        [0.85, 0.09, 0.01],  // dark red
        [0.80, 0.02, 0.00],  // dark red
    ];

    const idx = Math.floor(v * (lut.length - 1));
    const t = (v * (lut.length - 1)) - idx;
    const nextIdx = Math.min(idx + 1, lut.length - 1);

    const c1 = lut[idx];
    const c2 = lut[nextIdx];

    // Clamp to valid RGB range [0, 1]
    const r = Math.max(0, Math.min(1, c1[0] * (1 - t) + c2[0] * t));
    const g = Math.max(0, Math.min(1, c1[1] * (1 - t) + c2[1] * t));
    const b = Math.max(0, Math.min(1, c1[2] * (1 - t) + c2[2] * t));

    return { r, g, b };
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
