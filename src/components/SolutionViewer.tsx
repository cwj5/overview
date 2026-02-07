import { useState, useEffect } from 'react';
import { SCALAR_FIELDS, type ScalarField, formatValue, computeScalarField, getFieldStats } from '../utils/solutionData';
import { type ColorScheme } from '../utils/colorMapping';
import { ColorLegend } from './ColorLegend';
import type { GridItem } from '../types/grids';
import type { Plot3DSolution } from '../types/plot3d';
import './SolutionViewer.css';

interface SolutionViewerProps {
    selectedGrid: GridItem | null;
    onScalarFieldChange?: (field: ScalarField) => void;
    onColorSchemeChange?: (scheme: ColorScheme) => void;
}

export function SolutionViewer({ selectedGrid, onScalarFieldChange, onColorSchemeChange }: SolutionViewerProps) {
    const [selectedField, setSelectedField] = useState<ScalarField>('none');
    const [colorScheme, setColorScheme] = useState<ColorScheme>('viridis');
    const [fieldStats, setFieldStats] = useState<{ min: number, max: number, mean: number, stdDev: number } | null>(null);

    const hasSolution = selectedGrid?.solution !== undefined;

    // Compute field stats when selection changes
    useEffect(() => {
        if (!hasSolution || !selectedGrid?.solution || selectedField === 'none') {
            setFieldStats(null);
            return;
        }

        const solution = selectedGrid.solution as Plot3DSolution;
        const values = computeScalarField(solution, selectedField);
        const stats = getFieldStats(values);
        setFieldStats(stats);
    }, [selectedField, hasSolution, selectedGrid]);

    const handleFieldChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
        const field = e.target.value as ScalarField;
        setSelectedField(field);
        onScalarFieldChange?.(field);
    };

    const handleColorSchemeChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
        const scheme = e.target.value as ColorScheme;
        setColorScheme(scheme);
        onColorSchemeChange?.(scheme);
    };

    if (!hasSolution) {
        return (
            <div style={{
                padding: '12px',
                background: '#1f2937',
                borderRadius: '6px',
                fontSize: '12px',
                color: '#94a3b8'
            }}>
                <strong style={{ display: 'block', marginBottom: '6px' }}>Solution Visualization</strong>
                No solution data loaded for this grid
            </div>
        );
    }

    return (
        <div style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '12px',
            padding: '12px',
            background: '#1f2937',
            borderRadius: '6px',
            fontSize: '12px'
        }}>
            <strong style={{ textTransform: 'uppercase', letterSpacing: '0.08em', fontSize: '11px' }}>
                Solution Visualization
            </strong>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                <label style={{ fontSize: '12px', color: '#cbd5e1' }}>
                    <strong>Field:</strong>
                </label>
                <select
                    value={selectedField}
                    onChange={handleFieldChange}
                    style={{
                        padding: '6px',
                        background: '#111827',
                        color: '#e2e8f0',
                        border: '1px solid #374151',
                        borderRadius: '4px',
                        fontSize: '12px',
                        cursor: 'pointer'
                    }}
                >
                    {SCALAR_FIELDS.map(field => (
                        <option key={field.field} value={field.field}>
                            {field.name} ({field.unit})
                        </option>
                    ))}
                </select>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px' }}>
                <label style={{ fontSize: '12px', color: '#cbd5e1' }}>
                    <strong>Color Scheme:</strong>
                </label>
                <select
                    value={colorScheme}
                    onChange={handleColorSchemeChange}
                    style={{
                        padding: '6px',
                        background: '#111827',
                        color: '#e2e8f0',
                        border: '1px solid #374151',
                        borderRadius: '4px',
                        fontSize: '12px',
                        cursor: 'pointer'
                    }}
                >
                    <option value="viridis">Viridis (Perceptual)</option>
                    <option value="turbo">Turbo (Google)</option>
                    <option value="rainbow">Rainbow</option>
                    <option value="hot">Hot (Fire)</option>
                    <option value="grayscale">Grayscale</option>
                </select>
            </div>

            {fieldStats && selectedField !== 'none' && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
                    <ColorLegend
                        min={fieldStats.min}
                        max={fieldStats.max}
                        colorScheme={colorScheme}
                        orientation="horizontal"
                        numTicks={5}
                        label={SCALAR_FIELDS.find(f => f.field === selectedField)?.name}
                    />

                    <div style={{
                        display: 'grid',
                        gridTemplateColumns: '1fr 1fr',
                        gap: '8px',
                        fontSize: '11px'
                    }}>
                        <div>
                            <div style={{ color: '#94a3b8' }}>Min</div>
                            <div style={{ color: '#e2e8f0', fontWeight: 'bold' }}>
                                {formatValue(fieldStats.min)}
                            </div>
                        </div>
                        <div>
                            <div style={{ color: '#94a3b8' }}>Max</div>
                            <div style={{ color: '#e2e8f0', fontWeight: 'bold' }}>
                                {formatValue(fieldStats.max)}
                            </div>
                        </div>
                        <div>
                            <div style={{ color: '#94a3b8' }}>Mean</div>
                            <div style={{ color: '#e2e8f0', fontWeight: 'bold' }}>
                                {formatValue(fieldStats.mean)}
                            </div>
                        </div>
                        <div>
                            <div style={{ color: '#94a3b8' }}>Std Dev</div>
                            <div style={{ color: '#e2e8f0', fontWeight: 'bold' }}>
                                {formatValue(fieldStats.stdDev)}
                            </div>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}
