import { useState, useEffect } from 'react';
import { SCALAR_FIELDS, type ScalarField, formatValue, computeScalarField, getFieldStats } from '../utils/solutionData';
import { mapValueToColor } from '../utils/colorMapping';
import type { GridItem } from '../types/grids';
import type { Plot3DSolution } from '../types/plot3d';
import './SolutionViewer.css';

interface SolutionViewerProps {
    selectedGrid: GridItem | null;
    onScalarFieldChange?: (field: ScalarField) => void;
}

export function SolutionViewer({ selectedGrid, onScalarFieldChange }: SolutionViewerProps) {
    const [selectedField, setSelectedField] = useState<ScalarField>('density');
    const [fieldStats, setFieldStats] = useState<{ min: number, max: number, mean: number, stdDev: number } | null>(null);

    const hasSolution = selectedGrid?.solution !== undefined;

    // Compute field stats when selection changes
    useEffect(() => {
        if (!hasSolution || !selectedGrid?.solution) {
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
                <label style={{ fontSize: '12px', color: '#cbd5f5' }}>
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

            {fieldStats && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                    <ColorBar min={fieldStats.min} max={fieldStats.max} />

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

interface ColorBarProps {
    min: number;
    max: number;
}

function ColorBar({ min, max }: ColorBarProps) {
    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '4px' }}>
            <div
                style={{
                    height: '20px',
                    background: `linear-gradient(to right, ${Array.from({ length: 11 }, (_, i) => {
                        const value = i / 10;
                        const color = mapValueToColor(value, 'viridis');
                        const hex = `rgb(${Math.round(color.r * 255)}, ${Math.round(color.g * 255)}, ${Math.round(color.b * 255)})`;
                        return `${hex} ${i * 10}%`;
                    }).join(',')})`,
                    borderRadius: '4px',
                    border: '1px solid #374151'
                }}
            />
            <div style={{
                display: 'flex',
                justifyContent: 'space-between',
                fontSize: '10px',
                color: '#94a3b8'
            }}>
                <span>{formatValue(min)}</span>
                <span>{formatValue((min + max) / 2)}</span>
                <span>{formatValue(max)}</span>
            </div>
        </div>
    );
}
