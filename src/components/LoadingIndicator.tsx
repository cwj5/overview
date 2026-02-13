import './LoadingIndicator.css';
import { logger } from '../utils/logger';

interface LoadingIndicatorProps {
    isLoading: boolean;
    message?: string;
}

export function LoadingIndicator({ isLoading, message = 'Loading...' }: LoadingIndicatorProps) {
    // Debug logging
    logger.debug(`LoadingIndicator: isLoading=${isLoading}, message="${message}"`, 'LoadingIndicator');

    if (!isLoading) return null;

    return (
        <div className="loading-overlay">
            <div className="loading-container">
                <div className="loading-spinner">
                    <div className="spinner-ring"></div>
                    <div className="spinner-ring"></div>
                    <div className="spinner-ring"></div>
                </div>
                <p className="loading-message">{message}</p>
            </div>
        </div>
    );
}
