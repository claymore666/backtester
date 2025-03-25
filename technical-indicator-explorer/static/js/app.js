/**
 * Technical Indicator Explorer
 * Frontend JavaScript
 */

// Global objects
let indicatorChart = null;
let currentAsset = null;
let currentInterval = null;
let currentIndicator = null;
let currentParams = null;
let allIntervals = ['1m', '3m', '5m', '15m', '30m', '1h', '2h', '4h', '6h', '8h', '12h', '1d', '3d', '1w', '1M'];

// DOM Elements
const assetTable = document.getElementById('assetTable');
const assetLoader = document.getElementById('assetLoader');
const assetSearch = document.getElementById('assetSearch');
const indicatorTitle = document.getElementById('indicatorTitle');
const indicatorData = document.getElementById('indicatorData');
const parameterSelect = document.getElementById('parameterSelect');
const indicatorLoader = document.querySelector('.indicator-loader');

// Modal
const indicatorModal = new bootstrap.Modal(document.getElementById('indicatorModal'));

/**
 * Initialize the application
 */
document.addEventListener('DOMContentLoaded', function() {
    loadAssets();
    
    // Setup search functionality
    assetSearch.addEventListener('input', filterAssets);
    
    // Setup parameter select change event
    parameterSelect.addEventListener('change', function() {
        currentParams = JSON.parse(this.value);
        loadIndicatorData();
    });
});

/**
 * Load all available assets
 */
async function loadAssets() {
    try {
        assetLoader.classList.remove('d-none');
        const response = await fetch('/api/assets');
        
        if (!response.ok) {
            throw new Error('Failed to load assets');
        }
        
        const assets = await response.json();
        renderAssetTable(assets);
    } catch (error) {
        console.error('Error loading assets:', error);
        document.querySelector('#assetTable tbody').innerHTML = 
            `<tr><td colspan="5" class="text-danger">Error loading assets: ${error.message}</td></tr>`;
    } finally {
        assetLoader.classList.add('d-none');
    }
}

/**
 * Render the table of assets
 */
function renderAssetTable(assets) {
    if (assets.length === 0) {
        document.querySelector('#assetTable tbody').innerHTML = 
            '<tr><td colspan="5" class="text-center">No assets found</td></tr>';
        return;
    }
    
    // Sort assets alphabetically
    assets.sort((a, b) => a.symbol.localeCompare(b.symbol));
    
    let html = '';
    for (const asset of assets) {
        // Create interval badges
        const intervalBadges = allIntervals.map(interval => {
            const isAvailable = asset.intervals.includes(interval);
            return `<span class="badge interval-badge ${isAvailable ? 'available' : 'unavailable'}">${interval}</span>`;
        }).join('');
        
        // Format dates
        const firstDate = asset.first_candle ? new Date(asset.first_candle).toLocaleDateString() : 'N/A';
        const lastDate = asset.last_candle ? new Date(asset.last_candle).toLocaleDateString() : 'N/A';
        
        html += `
            <tr data-symbol="${asset.symbol}" class="asset-row">
                <td>${asset.symbol}</td>
                <td>${intervalBadges}</td>
                <td>${asset.candle_count?.toLocaleString() || 0}</td>
                <td>${firstDate} - ${lastDate}</td>
                <td>
                    <button class="btn btn-sm btn-primary show-indicators-btn" data-symbol="${asset.symbol}">
                        Show Indicators
                    </button>
                </td>
            </tr>
            <tr class="indicator-row d-none" data-symbol="${asset.symbol}">
                <td colspan="5" class="p-3 bg-light">
                    <div class="indicator-container">
                        <div class="text-center">
                            <div class="spinner-border spinner-border-sm" role="status">
                                <span class="visually-hidden">Loading...</span>
                            </div>
                            <span class="ms-2">Loading indicators...</span>
                        </div>
                    </div>
                </td>
            </tr>
        `;
    }
    
    document.querySelector('#assetTable tbody').innerHTML = html;
    
    // Add click event to show indicator buttons
    document.querySelectorAll('.show-indicators-btn').forEach(button => {
        button.addEventListener('click', function(e) {
            e.stopPropagation();
            const symbol = this.dataset.symbol;
            toggleIndicatorRow(symbol);
        });
    });
}

/**
 * Toggle indicator row visibility
 */
function toggleIndicatorRow(symbol) {
    const indicatorRow = document.querySelector(`.indicator-row[data-symbol="${symbol}"]`);
    
    // If row is already visible, just hide it
    if (!indicatorRow.classList.contains('d-none')) {
        indicatorRow.classList.add('d-none');
        return;
    }
    
    // Hide all other indicator rows
    document.querySelectorAll('.indicator-row').forEach(row => {
        row.classList.add('d-none');
    });
    
    // Show this row
    indicatorRow.classList.remove('d-none');
    
    // Load indicators if not already loaded
    if (indicatorRow.querySelector('.indicator-container').dataset.loaded !== 'true') {
        loadAssetDetails(symbol, indicatorRow.querySelector('.indicator-container'));
    }
}

/**
 * Filter assets based on search input
 */
function filterAssets() {
    const searchTerm = assetSearch.value.trim().toLowerCase();
    document.querySelectorAll('.asset-row').forEach(row => {
        const symbol = row.dataset.symbol.toLowerCase();
        const indicatorRow = document.querySelector(`.indicator-row[data-symbol="${row.dataset.symbol}"]`);
        
        if (symbol.includes(searchTerm)) {
            row.classList.remove('d-none');
            // Also check if indicator row was visible
            if (!indicatorRow.classList.contains('d-none')) {
                indicatorRow.classList.remove('d-none');
            }
        } else {
            row.classList.add('d-none');
            indicatorRow.classList.add('d-none');
        }
    });
}

/**
 * Load details for a specific asset
 */
async function loadAssetDetails(symbol, container) {
    try {
        currentAsset = symbol;
        
        const response = await fetch(`/api/asset/${symbol}`);
        
        if (!response.ok) {
            throw new Error('Failed to load asset details');
        }
        
        const data = await response.json();
        renderAssetIndicators(data, container);
        
        // Mark as loaded
        container.dataset.loaded = 'true';
    } catch (error) {
        console.error('Error loading asset details:', error);
        container.innerHTML = `<div class="alert alert-danger">Error loading details: ${error.message}</div>`;
    }
}

/**
 * Render asset indicators by interval
 */
function renderAssetIndicators(data, container) {
    // Order intervals by common order
    const timeframeOrder = {
        '1m': 1, '3m': 2, '5m': 3, '15m': 4, '30m': 5,
        '1h': 6, '2h': 7, '4h': 8, '6h': 9, '8h': 10, '12h': 11,
        '1d': 12, '3d': 13, '1w': 14, '1M': 15
    };
    
    data.intervals.sort((a, b) => {
        return (timeframeOrder[a.interval] || 999) - (timeframeOrder[b.interval] || 999);
    });
    
    let html = `<div class="row">`;
    
    // Create columns for intervals with indicators
    data.intervals.forEach(interval => {
        // Filter indicators for this interval
        const intervalIndicators = data.configured_indicators.filter(i => i.interval === interval.interval);
        
        if (intervalIndicators.length === 0) {
            return; // Skip intervals with no indicators
        }
        
        // Group indicators by type
        const indicatorsByType = {};
        intervalIndicators.forEach(indicator => {
            if (!indicatorsByType[indicator.type]) {
                indicatorsByType[indicator.type] = [];
            }
            indicatorsByType[indicator.type].push(indicator);
        });
        
        html += `
            <div class="col-md-4 mb-3">
                <div class="card h-100">
                    <div class="card-header">
                        <h6 class="mb-0">${interval.interval} Interval</h6>
                        <small class="text-muted">${interval.candle_count.toLocaleString()} candles</small>
                    </div>
                    <div class="card-body">
        `;
        
        // Order indicator types
        const typeOrder = [
            'oscillator', 'overlap', 'volatility', 'volume', 'pattern'
        ];
        
        // Sort indicator types
        const sortedTypes = Object.keys(indicatorsByType).sort((a, b) => {
            const aIndex = typeOrder.indexOf(a);
            const bIndex = typeOrder.indexOf(b);
            return (aIndex !== -1 ? aIndex : 999) - (bIndex !== -1 ? bIndex : 999);
        });
        
        // Create indicator sections grouped by type
        sortedTypes.forEach(type => {
            const indicators = indicatorsByType[type];
            
            // Capitalize type name
            const typeDisplay = type.charAt(0).toUpperCase() + type.slice(1);
            
            html += `
                <div class="indicator-section">
                    <div class="indicator-type">${typeDisplay}:</div>
                    <div class="indicator-list">
            `;
            
            // Sort indicators by name
            indicators.sort((a, b) => a.name.localeCompare(b.name));
            
            // Create indicator links
            indicators.forEach(indicator => {
                const paramsStr = JSON.stringify(indicator.parameters);
                html += `
                    <div class="indicator-item" 
                         data-symbol="${data.symbol}"
                         data-interval="${interval.interval}"
                         data-indicator="${indicator.name}"
                         data-params='${paramsStr}'>
                        ${indicator.name}
                        <small class="text-muted">
                            (${Object.keys(indicator.parameters).length > 0 ? 
                                Object.entries(indicator.parameters).map(([k, v]) => `${k}:${v}`).join(',') : 
                                'default'})
                        </small>
                    </div>
                `;
            });
            
            html += `
                    </div>
                </div>
            `;
        });
        
        html += `
                    </div>
                </div>
            </div>
        `;
    });
    
    html += `</div>`;
    
    // If no indicators found
    if (data.configured_indicators.length === 0) {
        html = `<div class="alert alert-info">No indicators configured for ${data.symbol}</div>`;
    }
    
    container.innerHTML = html;
    
    // Add event listeners to indicators
    document.querySelectorAll('.indicator-item').forEach(element => {
        element.addEventListener('click', function() {
            const symbol = this.dataset.symbol;
            const interval = this.dataset.interval;
            const indicator = this.dataset.indicator;
            const params = JSON.parse(this.dataset.params);
            
            loadIndicator(symbol, interval, indicator, params);
        });
    });
}

/**
 * Load indicator data and show modal
 */
async function loadIndicator(symbol, interval, indicatorName, params) {
    try {
        // Set current values
        currentAsset = symbol;
        currentInterval = interval;
        currentIndicator = indicatorName;
        currentParams = params;
        
        // Update modal title
        indicatorTitle.textContent = `${indicatorName} (${symbol}, ${interval})`;
        
        // Show modal
        indicatorModal.show();
        
        // Show loader
        indicatorLoader.classList.remove('d-none');
        
        // First, load all parameters variations for this indicator
        const response = await fetch(`/api/indicators/${symbol}/${interval}`);
        
        if (!response.ok) {
            throw new Error('Failed to load indicator list');
        }
        
        const data = await response.json();
        
        // Find this specific indicator in the data
        const indicator = data.find(ind => ind.name === indicatorName);
        
        if (!indicator) {
            throw new Error('Indicator not found in calculated data');
        }
        
        // Populate parameter select
        renderParameterOptions(indicator.parameters_variations);
        
        // Now load the indicator data
        await loadIndicatorData();
        
    } catch (error) {
        console.error('Error loading indicator:', error);
        indicatorData.innerHTML = `<tr><td colspan="2" class="text-danger">Error: ${error.message}</td></tr>`;
        indicatorLoader.classList.add('d-none');
    }
}

/**
 * Render parameter options in select dropdown
 */
function renderParameterOptions(parametersList) {
    parameterSelect.innerHTML = '';
    
    parametersList.forEach(params => {
        const option = document.createElement('option');
        option.value = JSON.stringify(params.parameters);
        
        // Create readable label for parameters
        let label = '';
        if (Object.keys(params.parameters).length === 0) {
            label = 'Default parameters';
        } else {
            label = Object.entries(params.parameters)
                .map(([key, value]) => `${key}: ${value}`)
                .join(', ');
        }
        label += ` (${params.count} points)`;
        
        option.textContent = label;
        
        // Select this option if it matches current params
        if (JSON.stringify(params.parameters) === JSON.stringify(currentParams)) {
            option.selected = true;
        }
        
        parameterSelect.appendChild(option);
    });
    
    // Update currentParams with first option if not already set
    if (!currentParams && parameterSelect.options.length > 0) {
        currentParams = JSON.parse(parameterSelect.options[0].value);
    }
}

/**
 * Load indicator data points
 */
async function loadIndicatorData() {
    try {
        indicatorLoader.classList.remove('d-none');
        
        // Encode parameters as query string
        const paramsStr = encodeURIComponent(JSON.stringify(currentParams));
        const url = `/api/indicator-data/${currentAsset}/${currentInterval}/${currentIndicator}?parameters=${paramsStr}&limit=100`;
        
        const response = await fetch(url);
        
        if (!response.ok) {
            throw new Error('Failed to load indicator data');
        }
        
        const data = await response.json();
        renderIndicatorData(data);
        
    } catch (error) {
        console.error('Error loading indicator data:', error);
        indicatorData.innerHTML = `<tr><td colspan="2" class="text-danger">Error: ${error.message}</td></tr>`;
        
        // Clear chart
        if (indicatorChart) {
            indicatorChart.destroy();
            indicatorChart = null;
        }
    } finally {
        indicatorLoader.classList.add('d-none');
    }
}

/**
 * Render indicator data in table and chart
 */
function renderIndicatorData(data) {
    if (data.length === 0) {
        indicatorData.innerHTML = `<tr><td colspan="2" class="text-center">No data available</td></tr>`;
        return;
    }
    
    // Sort data by time (oldest first for the chart)
    data.sort((a, b) => new Date(a.time) - new Date(b.time));
    
    // Render table (latest points first)
    const reversed = [...data].reverse();
    let tableHtml = '';
    
    for (const point of reversed) {
        const time = new Date(point.time).toLocaleString();
        let valueHtml = '';
        
        // Handle different value types
        if (typeof point.value === 'object') {
            valueHtml = Object.entries(point.value)
                .map(([key, val]) => `<div><strong>${key}:</strong> ${val}</div>`)
                .join('');
        } else {
            valueHtml = point.value;
        }
        
        tableHtml += `
            <tr>
                <td>${time}</td>
                <td>${valueHtml}</td>
            </tr>
        `;
    }
    
    indicatorData.innerHTML = tableHtml;
    
    // Render chart
    createChart(data);
}

/**
 * Create or update chart with indicator data
 */
function createChart(data) {
    const ctx = document.getElementById('indicatorChart').getContext('2d');
    
    // Destroy previous chart if exists
    if (indicatorChart) {
        indicatorChart.destroy();
    }
    
    // Prepare chart data
    const labels = data.map(d => new Date(d.time));
    
    // Create datasets based on value structure
    let datasets = [];
    
    if (data.length > 0) {
        // Check if value is an object with multiple values or a single value
        const sampleValue = data[0].value;
        
        if (typeof sampleValue === 'object') {
            // Create a dataset for each property
            const properties = Object.keys(sampleValue);
            
            // Predefined colors for consistent display
            const colors = [
                'rgba(75, 192, 192, 1)',
                'rgba(255, 99, 132, 1)',
                'rgba(54, 162, 235, 1)',
                'rgba(255, 206, 86, 1)',
                'rgba(153, 102, 255, 1)',
                'rgba(255, 159, 64, 1)'
            ];
            
            properties.forEach((prop, index) => {
                datasets.push({
                    label: prop,
                    data: data.map(d => d.value[prop]),
                    borderColor: colors[index % colors.length],
                    backgroundColor: colors[index % colors.length].replace('1)', '0.1)'),
                    borderWidth: 1.5,
                    tension: 0.1
                });
            });
        } else {
            // Single value dataset
            datasets = [{
                label: currentIndicator,
                data: data.map(d => d.value),
                borderColor: 'rgba(75, 192, 192, 1)',
                backgroundColor: 'rgba(75, 192, 192, 0.1)',
                borderWidth: 1.5,
                tension: 0.1
            }];
        }
    }
    
    // Create chart
    indicatorChart = new Chart(ctx, {
        type: 'line',
        data: {
            labels: labels,
            datasets: datasets
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            scales: {
                x: {
                    type: 'time',
                    time: {
                        unit: 'day',
                        displayFormats: {
                            day: 'MMM d'
                        }
                    },
                    title: {
                        display: true,
                        text: 'Date'
                    }
                },
                y: {
                    beginAtZero: false,
                    title: {
                        display: true,
                        text: 'Value'
                    }
                }
            },
            plugins: {
                tooltip: {
                    mode: 'index',
                    intersect: false
                },
                legend: {
                    display: datasets.length > 1
                }
            }
        }
    });
}
