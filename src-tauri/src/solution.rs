/// Solution data visualization and computation functions
use crate::plot3d::Plot3DSolution;

/// Color scheme types for visualization
#[derive(Debug, Clone)]
pub enum ColorScheme {
    Viridis,
    Turbo,
    Rainbow,
    Hot,
    Grayscale,
}

impl ColorScheme {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "viridis" => Some(ColorScheme::Viridis),
            "turbo" => Some(ColorScheme::Turbo),
            "rainbow" => Some(ColorScheme::Rainbow),
            "hot" => Some(ColorScheme::Hot),
            "grayscale" => Some(ColorScheme::Grayscale),
            _ => None,
        }
    }
}

/// Scalar field types
#[derive(Debug, Clone)]
pub enum ScalarField {
    Density,
    VelocityMagnitude,
    MomentumX,
    MomentumY,
    MomentumZ,
    Pressure,
    Energy,
}

impl ScalarField {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "density" => Some(ScalarField::Density),
            "velocity_magnitude" => Some(ScalarField::VelocityMagnitude),
            "momentum_x" => Some(ScalarField::MomentumX),
            "momentum_y" => Some(ScalarField::MomentumY),
            "momentum_z" => Some(ScalarField::MomentumZ),
            "pressure" => Some(ScalarField::Pressure),
            "energy" => Some(ScalarField::Energy),
            _ => None,
        }
    }
}

/// Compute a scalar field from solution data
#[allow(dead_code)]
pub fn compute_scalar_field(solution: &Plot3DSolution, field: ScalarField) -> Vec<f32> {
    let total_points = solution.rho.len();
    let mut result = Vec::with_capacity(total_points);

    match field {
        ScalarField::Density => {
            result = solution.rho.clone();
        }

        ScalarField::VelocityMagnitude => {
            // |V| = sqrt(u² + v² + w²)
            // where u = rhou/rho, v = rhov/rho, w = rhow/rho
            for i in 0..total_points {
                let rho = solution.rho[i];
                if rho > 0.0 {
                    let u = solution.rhou[i] / rho;
                    let v = solution.rhov[i] / rho;
                    let w = solution.rhow[i] / rho;
                    result.push((u * u + v * v + w * w).sqrt());
                } else {
                    result.push(0.0);
                }
            }
        }

        ScalarField::Pressure => {
            // p = (gamma - 1) * (rhoe - 0.5 * rho * (u² + v² + w²))
            // Use gamma from solution file if available, otherwise default to 1.4 (air)
            const DEFAULT_GAMMA: f32 = 1.4;
            for i in 0..total_points {
                let rho = solution.rho[i];
                if rho > 0.0 {
                    let gamma = solution
                        .gamma
                        .as_ref()
                        .map(|g| g[i])
                        .unwrap_or(DEFAULT_GAMMA);
                    let u = solution.rhou[i] / rho;
                    let v = solution.rhov[i] / rho;
                    let w = solution.rhow[i] / rho;
                    let kinetic_energy = 0.5 * rho * (u * u + v * v + w * w);
                    let internal_energy = solution.rhoe[i] - kinetic_energy;
                    result.push((gamma - 1.0) * internal_energy);
                } else {
                    result.push(0.0);
                }
            }
        }

        ScalarField::MomentumX => {
            result = solution.rhou.clone();
        }

        ScalarField::MomentumY => {
            result = solution.rhov.clone();
        }

        ScalarField::MomentumZ => {
            result = solution.rhow.clone();
        }

        ScalarField::Energy => {
            result = solution.rhoe.clone();
        }
    }

    result
}

/// Compute scalar field for the k=0 surface with optional decimation.
pub fn compute_scalar_field_surface(
    solution: &Plot3DSolution,
    field: ScalarField,
    decimation_factor: usize,
) -> Vec<f32> {
    let decimation = decimation_factor.max(1);
    let i = solution.dimensions.i as usize;
    let j = solution.dimensions.j as usize;
    let k_idx = 0usize;

    let i_decimated = ((i - 1) / decimation) + 1;
    let j_decimated = ((j - 1) / decimation) + 1;

    let mut values = Vec::with_capacity(i_decimated * j_decimated);

    for j_step in 0..j_decimated {
        let j_idx = (j_step * decimation).min(j - 1);
        for i_step in 0..i_decimated {
            let i_idx = (i_step * decimation).min(i - 1);
            let idx = k_idx * i * j + j_idx * i + i_idx;

            let value = match field {
                ScalarField::Density => solution.rho[idx],
                ScalarField::MomentumX => solution.rhou[idx],
                ScalarField::MomentumY => solution.rhov[idx],
                ScalarField::MomentumZ => solution.rhow[idx],
                ScalarField::Energy => solution.rhoe[idx],
                ScalarField::VelocityMagnitude => {
                    let rho = solution.rho[idx];
                    if rho > 0.0 {
                        let u = solution.rhou[idx] / rho;
                        let v = solution.rhov[idx] / rho;
                        let w = solution.rhow[idx] / rho;
                        (u * u + v * v + w * w).sqrt()
                    } else {
                        0.0
                    }
                }
                ScalarField::Pressure => {
                    const DEFAULT_GAMMA: f32 = 1.4;
                    let rho = solution.rho[idx];
                    if rho > 0.0 {
                        let gamma = solution
                            .gamma
                            .as_ref()
                            .map(|g| g[idx])
                            .unwrap_or(DEFAULT_GAMMA);
                        let u = solution.rhou[idx] / rho;
                        let v = solution.rhov[idx] / rho;
                        let w = solution.rhow[idx] / rho;
                        let kinetic_energy = 0.5 * rho * (u * u + v * v + w * w);
                        let internal_energy = solution.rhoe[idx] - kinetic_energy;
                        (gamma - 1.0) * internal_energy
                    } else {
                        0.0
                    }
                }
            };

            values.push(value);
        }
    }

    values
}

/// Color mapping function from normalized value [0, 1] to RGB
pub fn map_value_to_color(value: f32, scheme: &ColorScheme) -> (f32, f32, f32) {
    if !value.is_finite() {
        return (0.0, 0.0, 0.0);
    }
    let v = value.max(0.0).min(1.0);
    match scheme {
        ColorScheme::Viridis => viridis_color(v),
        ColorScheme::Turbo => turbo_color(v),
        ColorScheme::Rainbow => rainbow_color(v),
        ColorScheme::Hot => hot_color(v),
        ColorScheme::Grayscale => (v, v, v),
    }
}

fn viridis_color(v: f32) -> (f32, f32, f32) {
    let lut = [
        (0.267004, 0.004874, 0.329415),
        (0.282623, 0.140461, 0.469470),
        (0.253935, 0.265254, 0.529983),
        (0.206756, 0.371758, 0.553806),
        (0.163625, 0.471133, 0.558695),
        (0.127568, 0.566949, 0.550413),
        (0.134692, 0.658636, 0.517649),
        (0.266941, 0.748751, 0.440573),
        (0.477504, 0.821444, 0.318195),
        (0.741388, 0.873449, 0.149561),
        (0.993248, 0.906157, 0.143936),
    ];
    let idx = (v * (lut.len() - 1) as f32).floor() as usize;
    let t = (v * (lut.len() - 1) as f32) - idx as f32;
    let next_idx = (idx + 1).min(lut.len() - 1);
    let (r1, g1, b1) = lut[idx];
    let (r2, g2, b2) = lut[next_idx];
    (
        r1 * (1.0 - t) + r2 * t,
        g1 * (1.0 - t) + g2 * t,
        b1 * (1.0 - t) + b2 * t,
    )
}

fn turbo_color(v: f32) -> (f32, f32, f32) {
    // Google Turbo colormap sampled at 16 key points
    let lut = [
        (0.19, 0.07, 0.23), // dark purple/blue
        (0.21, 0.14, 0.42), // purple-blue
        (0.24, 0.26, 0.61), // blue
        (0.27, 0.38, 0.81), // cyan-blue
        (0.29, 0.50, 0.93), // cyan
        (0.28, 0.63, 0.94), // cyan-green
        (0.25, 0.74, 0.80), // green
        (0.42, 0.84, 0.54), // yellow-green
        (0.67, 0.90, 0.28), // yellow
        (0.89, 0.88, 0.12), // orange-yellow
        (1.00, 0.77, 0.06), // orange
        (1.00, 0.60, 0.03), // orange-red
        (0.97, 0.40, 0.02), // red-orange
        (0.92, 0.20, 0.01), // red
        (0.85, 0.09, 0.01), // dark red
        (0.80, 0.02, 0.00), // dark red
    ];
    let idx = (v * (lut.len() - 1) as f32).floor() as usize;
    let t = (v * (lut.len() - 1) as f32) - idx as f32;
    let next_idx = (idx + 1).min(lut.len() - 1);
    let (r1, g1, b1) = lut[idx];
    let (r2, g2, b2) = lut[next_idx];
    (
        (r1 * (1.0 - t) + r2 * t).max(0.0).min(1.0),
        (g1 * (1.0 - t) + g2 * t).max(0.0).min(1.0),
        (b1 * (1.0 - t) + b2 * t).max(0.0).min(1.0),
    )
}

fn rainbow_color(v: f32) -> (f32, f32, f32) {
    let (mut r, mut g, mut b) = (0.0, 0.0, 0.0);
    if v < 0.2 {
        r = 1.0;
        g = v / 0.2;
    } else if v < 0.4 {
        r = 1.0 - (v - 0.2) / 0.2;
        g = 1.0;
    } else if v < 0.6 {
        g = 1.0;
        b = (v - 0.4) / 0.2;
    } else if v < 0.8 {
        g = 1.0 - (v - 0.6) / 0.2;
        b = 1.0;
    } else {
        r = (v - 0.8) / 0.2;
        b = 1.0;
    }
    (r, g, b)
}

fn hot_color(v: f32) -> (f32, f32, f32) {
    if v < 0.33 {
        (v / 0.33, 0.0, 0.0)
    } else if v < 0.66 {
        (1.0, (v - 0.33) / 0.33, 0.0)
    } else {
        (1.0, 1.0, (v - 0.66) / 0.34)
    }
}

/// Compute vertex colors for a scalar field
pub fn compute_colors(values: &[f32], scheme: &ColorScheme) -> Vec<f32> {
    if values.is_empty() {
        return Vec::new();
    }

    // Find min/max using finite values only
    let mut min: Option<f32> = None;
    let mut max: Option<f32> = None;
    for &v in values.iter() {
        if !v.is_finite() {
            continue;
        }
        min = Some(match min {
            Some(current) => current.min(v),
            None => v,
        });
        max = Some(match max {
            Some(current) => current.max(v),
            None => v,
        });
    }

    let (min, max) = match (min, max) {
        (Some(min), Some(max)) => (min, max),
        _ => {
            // No finite values; return black
            return vec![0.0; values.len() * 3];
        }
    };

    let mut range = max - min;
    if !range.is_finite() || range <= 0.0 {
        range = 1.0;
    }

    // Generate colors
    let mut colors = Vec::with_capacity(values.len() * 3);
    for &v in values.iter() {
        let mut normalized = if v.is_finite() {
            (v - min) / range
        } else {
            0.0
        };
        if !normalized.is_finite() {
            normalized = 0.0;
        }
        let (r, g, b) = map_value_to_color(normalized, scheme);
        colors.push(r);
        colors.push(g);
        colors.push(b);
    }

    colors
}

/// Compute field statistics
#[allow(dead_code)]
pub struct FieldStats {
    pub min: f32,
    pub max: f32,
    pub mean: f32,
    pub std_dev: f32,
}

#[allow(dead_code)]
pub fn compute_field_stats(values: &[f32]) -> FieldStats {
    if values.is_empty() {
        return FieldStats {
            min: 0.0,
            max: 0.0,
            mean: 0.0,
            std_dev: 0.0,
        };
    }

    let mut min = values[0];
    let mut max = values[0];
    let mut sum = 0.0;

    for &v in values.iter() {
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
        sum += v;
    }

    let mean = sum / values.len() as f32;

    let mut sum_squared_diff = 0.0;
    for &v in values.iter() {
        let diff = v - mean;
        sum_squared_diff += diff * diff;
    }
    let std_dev = (sum_squared_diff / values.len() as f32).sqrt();

    FieldStats {
        min,
        max,
        mean,
        std_dev,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plot3d::GridDimensions;

    /// Helper to create a test solution
    fn create_test_solution(size: usize, include_gamma: bool) -> Plot3DSolution {
        let mut rho = Vec::with_capacity(size);
        let mut rhou = Vec::with_capacity(size);
        let mut rhov = Vec::with_capacity(size);
        let mut rhow = Vec::with_capacity(size);
        let mut rhoe = Vec::with_capacity(size);

        // Fill with test data
        for i in 0..size {
            let r = 1.0 + (i as f32) * 0.1;
            rho.push(r);
            rhou.push(0.5 * r);
            rhov.push(0.3 * r);
            rhow.push(0.2 * r);
            rhoe.push(2.5 * r);
        }

        let gamma = if include_gamma {
            Some((0..size).map(|i| 1.4 + (i as f32) * 0.01).collect())
        } else {
            None
        };

        Plot3DSolution {
            grid_index: 0,
            dimensions: GridDimensions { i: 2, j: 2, k: 1 },
            rho,
            rhou,
            rhov,
            rhow,
            rhoe,
            gamma,
            metadata: None,
        }
    }

    #[test]
    fn test_scalar_field_from_str() {
        assert!(matches!(
            ScalarField::from_str("density"),
            Some(ScalarField::Density)
        ));
        assert!(matches!(
            ScalarField::from_str("pressure"),
            Some(ScalarField::Pressure)
        ));
        assert!(matches!(
            ScalarField::from_str("velocity_magnitude"),
            Some(ScalarField::VelocityMagnitude)
        ));
        assert!(ScalarField::from_str("invalid").is_none());
    }

    #[test]
    fn test_compute_density_field() {
        let solution = create_test_solution(4, false);
        let result = compute_scalar_field(&solution, ScalarField::Density);

        assert_eq!(result.len(), 4);
        assert!((result[0] - 1.0).abs() < 1e-6);
        assert!((result[1] - 1.1).abs() < 1e-6);
        assert!((result[2] - 1.2).abs() < 1e-6);
        assert!((result[3] - 1.3).abs() < 1e-6);
    }

    #[test]
    fn test_compute_velocity_magnitude() {
        let solution = create_test_solution(4, false);
        let result = compute_scalar_field(&solution, ScalarField::VelocityMagnitude);

        assert_eq!(result.len(), 4);
        // For point 0: u=0.5, v=0.3, w=0.2 -> |V| = sqrt(0.25 + 0.09 + 0.04) = sqrt(0.38)
        let expected = (0.25_f32 + 0.09 + 0.04).sqrt();
        assert!((result[0] - expected).abs() < 1e-4);
    }

    #[test]
    fn test_compute_pressure_with_gamma() {
        let solution = create_test_solution(4, true);
        let result = compute_scalar_field(&solution, ScalarField::Pressure);

        assert_eq!(result.len(), 4);

        // For point 0: rho=1.0, u=0.5, v=0.3, w=0.2, rhoe=2.5, gamma=1.4
        // KE = 0.5 * 1.0 * (0.25 + 0.09 + 0.04) = 0.19
        // IE = 2.5 - 0.19 = 2.31
        // p = (1.4 - 1.0) * 2.31 = 0.924
        let rho = 1.0_f32;
        let ke = 0.5 * rho * (0.25 + 0.09 + 0.04);
        let ie = 2.5 - ke;
        let expected = (1.4 - 1.0) * ie;
        assert!((result[0] - expected).abs() < 1e-2);
    }

    #[test]
    fn test_compute_pressure_without_gamma() {
        let solution = create_test_solution(4, false);
        let result = compute_scalar_field(&solution, ScalarField::Pressure);

        assert_eq!(result.len(), 4);

        // Should use DEFAULT_GAMMA = 1.4
        let rho = 1.0_f32;
        let ke = 0.5 * rho * (0.25 + 0.09 + 0.04);
        let ie = 2.5 - ke;
        let expected = (1.4 - 1.0) * ie;
        assert!((result[0] - expected).abs() < 1e-2);
    }

    #[test]
    fn test_pressure_with_varying_gamma() {
        let solution = create_test_solution(2, true);
        let result = compute_scalar_field(&solution, ScalarField::Pressure);

        // Points should have different gamma values (1.4 and 1.41)
        // So pressures should be slightly different even with same flow pattern
        assert_ne!(result[0], result[1]);
    }

    #[test]
    fn test_compute_momentum_fields() {
        let solution = create_test_solution(4, false);

        let mom_x = compute_scalar_field(&solution, ScalarField::MomentumX);
        assert!((mom_x[0] - 0.5).abs() < 1e-6);

        let mom_y = compute_scalar_field(&solution, ScalarField::MomentumY);
        assert!((mom_y[0] - 0.3).abs() < 1e-6);

        let mom_z = compute_scalar_field(&solution, ScalarField::MomentumZ);
        assert!((mom_z[0] - 0.2).abs() < 1e-6);
    }

    #[test]
    fn test_compute_energy_field() {
        let solution = create_test_solution(4, false);
        let result = compute_scalar_field(&solution, ScalarField::Energy);

        assert_eq!(result.len(), 4);
        assert!((result[0] - 2.5).abs() < 1e-6);
        assert!((result[3] - 3.25).abs() < 1e-5);
    }

    #[test]
    fn test_zero_density_handling() {
        let mut solution = create_test_solution(2, false);
        solution.rho[0] = 0.0;

        let velocity = compute_scalar_field(&solution, ScalarField::VelocityMagnitude);
        assert_eq!(velocity[0], 0.0);

        let pressure = compute_scalar_field(&solution, ScalarField::Pressure);
        assert_eq!(pressure[0], 0.0);
    }

    #[test]
    fn test_compute_field_stats() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = compute_field_stats(&values);

        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 5.0);
        assert_eq!(stats.mean, 3.0);
        assert!((stats.std_dev - 1.4142).abs() < 0.001);
    }

    #[test]
    fn test_field_stats_single_value() {
        let values = vec![42.0];
        let stats = compute_field_stats(&values);

        assert_eq!(stats.min, 42.0);
        assert_eq!(stats.max, 42.0);
        assert_eq!(stats.mean, 42.0);
        assert_eq!(stats.std_dev, 0.0);
    }

    #[test]
    fn test_field_stats_uniform_values() {
        let values = vec![3.14, 3.14, 3.14, 3.14];
        let stats = compute_field_stats(&values);

        assert_eq!(stats.min, 3.14);
        assert_eq!(stats.max, 3.14);
        assert_eq!(stats.mean, 3.14);
        assert!((stats.std_dev).abs() < 1e-6);
    }

    #[test]
    fn test_map_value_to_color_bounds() {
        // Test clamping
        let (r, g, b) = map_value_to_color(-0.5, &ColorScheme::Viridis);
        assert!(r >= 0.0 && r <= 1.0);
        assert!(g >= 0.0 && g <= 1.0);
        assert!(b >= 0.0 && b <= 1.0);

        let (r, g, b) = map_value_to_color(1.5, &ColorScheme::Viridis);
        assert!(r >= 0.0 && r <= 1.0);
        assert!(g >= 0.0 && g <= 1.0);
        assert!(b >= 0.0 && b <= 1.0);
    }

    #[test]
    fn test_map_value_to_color_range() {
        // Test typical values
        let (r0, g0, b0) = map_value_to_color(0.0, &ColorScheme::Viridis);
        let (r1, g1, b1) = map_value_to_color(1.0, &ColorScheme::Viridis);

        // Colors should be different at extremes
        assert!(
            (r0 - r1).abs() > 0.1 || (g0 - g1).abs() > 0.1 || (b0 - b1).abs() > 0.1,
            "Colors at 0 and 1 should be visibly different"
        );
    }

    #[test]
    fn test_compute_colors() {
        let solution = create_test_solution(4, false);
        let field_values = compute_scalar_field(&solution, ScalarField::Density);
        let colors = compute_colors(&field_values, &ColorScheme::Viridis);

        // Should have 3 color components (RGB) per point
        assert_eq!(colors.len(), 4 * 3);

        // All values should be in [0, 1]
        for &c in &colors {
            assert!(c >= 0.0 && c <= 1.0, "Color value {} out of range", c);
        }
    }

    #[test]
    fn test_compute_colors_empty() {
        let values: Vec<f32> = vec![];
        let colors = compute_colors(&values, &ColorScheme::Viridis);
        assert_eq!(colors.len(), 0);
    }
}
