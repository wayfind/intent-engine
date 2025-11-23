// Global state
let currentFilter = 'all';
let currentTaskId = null;
let currentTask = null;
let dashboardWebSocket = null;
let onlineProjects = new Map(); // project_path → project_info (online projects)
let isCurrentProjectOffline = false; // Track if current project is in offline/read-only mode

// WebSocket reconnection state
let wsReconnectAttempts = 0;
const WS_RECONNECT_DELAYS = [1000, 2000, 4000, 8000, 16000, 32000]; // Exponential backoff: 1s, 2s, 4s, 8s, 16s, 32s (max)
let wsHeartbeatTimer = null;
const WS_HEARTBEAT_TIMEOUT = 90000; // 90 seconds (protocol spec)
const PROTOCOL_VERSION = "1.0"; // Intent-Engine Protocol version

// LocalStorage heartbeat - check offline projects periodically
let storageHeartbeatTimer = null;
const STORAGE_HEARTBEAT_INTERVAL = 30000; // 30 seconds

// Initialize on page load
document.addEventListener('DOMContentLoaded', async () => {
    console.log('CORTEX ENGINE initializing...');

    // Configure marked.js
    marked.setOptions({
        gfm: true,
        breaks: true,
        highlight: (code, lang) => {
            if (lang && hljs.getLanguage(lang)) {
                try {
                    return hljs.highlight(code, { language: lang }).value;
                } catch (e) {
                    console.error('Highlight error:', e);
                }
            }
            return hljs.highlightAuto(code).value;
        }
    });

    // Connect to Dashboard WebSocket for real-time project updates
    connectToDashboardWebSocket();

    // Start localStorage heartbeat to check offline projects
    startStorageHeartbeat();

    // Load project info
    await loadProjectInfo();

    // Load tasks
    await loadTasks();

    // Load current task if exists
    await loadCurrentTask();

    // Start periodic polling for CLI/MCP sync (every 15 seconds)
    setInterval(async () => {
        try {
            await loadTasks(currentFilter);
            if (currentTaskId) {
                await loadTaskDetail(currentTaskId);
            }
            await loadCurrentTask();
        } catch (e) {
            console.error('Periodic refresh failed:', e);
        }
    }, 15000);
});

// Protocol v1.0: Send goodbye when page closes
window.addEventListener('beforeunload', () => {
    if (dashboardWebSocket && dashboardWebSocket.readyState === WebSocket.OPEN) {
        try {
            const goodbyeMessage = {
                version: PROTOCOL_VERSION,
                type: 'goodbye',
                payload: {
                    reason: 'Page closing'
                },
                timestamp: new Date().toISOString()
            };
            dashboardWebSocket.send(JSON.stringify(goodbyeMessage));
            console.log('✓ Sent goodbye message (page closing)');
        } catch (e) {
            console.error('Failed to send goodbye message:', e);
        }
    }
});

// Safe Markdown rendering
window.renderMarkdown = (md) => {
    if (!md) return '';
    try {
        const html = marked.parse(md);
        return DOMPurify.sanitize(html);
    } catch (e) {
        console.error('Markdown render error:', e);
        return '<p class="text-red-500">ERROR_RENDERING_DATA_STREAM</p>';
    }
};

// ============================================================================
// LocalStorage Management for Projects
// ============================================================================

const PROJECT_STORAGE_KEY = 'intent-engine-projects';

function loadProjectsFromStorage() {
    try {
        const stored = localStorage.getItem(PROJECT_STORAGE_KEY);
        return stored ? JSON.parse(stored) : [];
    } catch (e) {
        console.error('Failed to load projects from storage:', e);
        return [];
    }
}

function saveProjectsToStorage(projects) {
    try {
        localStorage.setItem(PROJECT_STORAGE_KEY, JSON.stringify(projects));
    } catch (e) {
        console.error('Failed to save projects to storage:', e);
    }
}

function addProjectToStorage(project) {
    const projects = loadProjectsFromStorage();
    // Check if project already exists
    const existingIndex = projects.findIndex(p => p.path === project.path);
    if (existingIndex >= 0) {
        // Update existing project
        projects[existingIndex] = project;
    } else {
        // Add new project
        projects.push(project);
    }
    saveProjectsToStorage(projects);
}

function removeProjectFromStorage(projectPath) {
    const projects = loadProjectsFromStorage();
    const filtered = projects.filter(p => p.path !== projectPath);
    saveProjectsToStorage(filtered);
}

// ============================================================================
// LocalStorage Heartbeat - Detect Offline Projects Coming Back Online
// ============================================================================

function startStorageHeartbeat() {
    // Initial check
    checkOfflineProjects();

    // Set up periodic checks
    storageHeartbeatTimer = setInterval(checkOfflineProjects, STORAGE_HEARTBEAT_INTERVAL);
    console.log('📦 LocalStorage heartbeat started (checking every 30s)');
}

async function checkOfflineProjects() {
    const storedProjects = loadProjectsFromStorage();

    // Find projects that are stored but not currently online
    const offlineProjects = storedProjects.filter(p => !onlineProjects.has(p.path));

    if (offlineProjects.length === 0) {
        return; // All stored projects are already online
    }

    console.log(`🔍 Checking ${offlineProjects.length} offline project(s)...`);

    // Check each offline project's health
    for (const project of offlineProjects) {
        try {
            // Try to fetch health endpoint (Dashboard must be running on port 11391)
            const response = await fetch('http://127.0.0.1:11391/api/health', {
                method: 'GET',
                signal: AbortSignal.timeout(2000) // 2 second timeout
            });

            if (response.ok) {
                // Dashboard is running! Check if our project is now online
                const infoResponse = await fetch('http://127.0.0.1:11391/api/info');
                if (infoResponse.ok) {
                    const info = await infoResponse.json();

                    // If this project matches the running Dashboard's project
                    if (info.path === project.path) {
                        console.log(`✓ Project "${project.name}" is now online!`);

                        // Manually update onlineProjects Map since WebSocket may not send
                        // project_online message when reconnecting (it only sends "init" with empty array)
                        onlineProjects.set(project.path, project);

                        // Refresh the UI to show updated status (now with green dot)
                        renderProjectTabs();
                    }
                }
            }
        } catch (error) {
            // Dashboard not responding or timeout - project still offline
            // This is expected for offline projects, no need to log
        }
    }
}

// ============================================================================
// WebSocket Connection for Real-Time Project Updates
// ============================================================================

function connectToDashboardWebSocket() {
    const wsUrl = `ws://${window.location.host}/ws/ui`;
    console.log(`Connecting to Dashboard WebSocket (attempt ${wsReconnectAttempts + 1}):`, wsUrl);

    // Clear existing heartbeat timer
    if (wsHeartbeatTimer) {
        clearTimeout(wsHeartbeatTimer);
        wsHeartbeatTimer = null;
    }

    dashboardWebSocket = new WebSocket(wsUrl);

    dashboardWebSocket.onopen = async () => {
        console.log('✓ Dashboard WebSocket connected');
        // Reset reconnect attempts on successful connection
        wsReconnectAttempts = 0;
        // Hide connection warning banner
        hideConnectionWarning();
        // Start heartbeat timeout timer
        resetHeartbeatTimer();

        // Protocol v1.0: Send hello message for handshake
        const helloMessage = {
            version: PROTOCOL_VERSION,
            type: 'hello',
            payload: {
                entity_type: 'web_ui',
                capabilities: null
            },
            timestamp: new Date().toISOString()
        };
        dashboardWebSocket.send(JSON.stringify(helloMessage));
        console.log('✓ Sent hello message');

        // WebSocket will send welcome, then init
        console.log('✓ Waiting for welcome and init messages...');
    };

    dashboardWebSocket.onmessage = (event) => {
        // Reset heartbeat timer on any message
        resetHeartbeatTimer();

        try {
            const message = JSON.parse(event.data);
            handleDashboardMessage(message);
        } catch (e) {
            console.error('Failed to parse WebSocket message:', e);
        }
    };

    dashboardWebSocket.onerror = (error) => {
        console.error('✗ Dashboard WebSocket error:', error);
    };

    dashboardWebSocket.onclose = () => {
        console.log('✗ Dashboard WebSocket closed');

        // Clear heartbeat timer
        if (wsHeartbeatTimer) {
            clearTimeout(wsHeartbeatTimer);
            wsHeartbeatTimer = null;
        }

        // Mark all projects as offline (gray lights)
        onlineProjects.clear();
        renderProjectTabs();

        // Infinite reconnection with exponential backoff + jitter
        const delayIndex = Math.min(wsReconnectAttempts, WS_RECONNECT_DELAYS.length - 1);
        const baseDelay = WS_RECONNECT_DELAYS[delayIndex];

        // Add jitter: ±25% random variance to prevent thundering herd
        const jitter = baseDelay * 0.25 * (Math.random() * 2 - 1); // Range: -25% to +25%
        const delay = Math.max(0, baseDelay + jitter);

        console.log(`⟳ Reconnecting in ${(delay/1000).toFixed(1)}s... (attempt ${wsReconnectAttempts + 1}, base: ${baseDelay/1000}s + jitter: ${(jitter/1000).toFixed(1)}s)`);

        // Show reconnecting banner
        showReconnectingBanner(wsReconnectAttempts + 1, delay);

        wsReconnectAttempts++;
        setTimeout(connectToDashboardWebSocket, delay);
    };
}

function resetHeartbeatTimer() {
    // Clear existing timer
    if (wsHeartbeatTimer) {
        clearTimeout(wsHeartbeatTimer);
    }

    // Set new timer - if no message received in 90s, consider connection dead
    wsHeartbeatTimer = setTimeout(() => {
        console.warn('⚠ WebSocket heartbeat timeout - no message received for 90s');
        if (dashboardWebSocket && dashboardWebSocket.readyState === WebSocket.OPEN) {
            dashboardWebSocket.close();
        }
    }, WS_HEARTBEAT_TIMEOUT);
}

// ============================================================================
// WebSocket UI Feedback Functions
// ============================================================================

function showReconnectingBanner(attempt, delay) {
    const banner = document.getElementById('connection-status-banner');
    if (!banner) return;

    banner.className = 'bg-yellow-900/30 border-b border-yellow-600/50 px-6 py-3';
    banner.innerHTML = `
        <div class="flex items-center gap-3">
            <div class="text-yellow-300 text-xl animate-spin">⟳</div>
            <div class="flex-1">
                <div class="font-mono text-sm text-yellow-300 font-bold tracking-wider">RECONNECTING...</div>
                <div class="font-mono text-xs text-yellow-400/80 mt-0.5">
                    Connection lost. Retrying in ${(delay/1000).toFixed(1)}s (attempt ${attempt})
                </div>
            </div>
        </div>
    `;
    banner.classList.remove('hidden');
}

function showConnectionFailedBanner() {
    const banner = document.getElementById('connection-status-banner');
    if (!banner) return;

    banner.className = 'bg-red-900/30 border-b border-red-600/50 px-6 py-3';
    banner.innerHTML = `
        <div class="flex items-center gap-3">
            <svg class="w-5 h-5 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"></path>
            </svg>
            <div class="flex-1">
                <div class="font-mono text-sm text-red-300 font-bold tracking-wider">⚠️ CONNECTION FAILED</div>
                <div class="font-mono text-xs text-red-400/80 mt-0.5">
                    Dashboard server is not responding. Please refresh the page or restart the Dashboard.
                </div>
            </div>
            <button onclick="window.location.reload()" class="px-4 py-2 bg-red-600 hover:bg-red-700 rounded font-mono text-xs text-white font-bold transition-colors uppercase tracking-wider">
                REFRESH
            </button>
        </div>
    `;
    banner.classList.remove('hidden');
}

function hideConnectionWarning() {
    const banner = document.getElementById('connection-status-banner');
    if (banner) {
        banner.classList.add('hidden');
    }
}

function showErrorBanner(message) {
    const banner = document.getElementById('connection-status-banner');
    if (!banner) return;

    banner.className = 'bg-red-900/30 border-b border-red-600/50 px-6 py-3';
    banner.innerHTML = `
        <div class="flex items-center gap-3">
            <svg class="w-5 h-5 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
            </svg>
            <div class="flex-1">
                <div class="font-mono text-sm text-red-300 font-bold tracking-wider">ERROR</div>
                <div class="font-mono text-xs text-red-400/80 mt-0.5">
                    ${message}
                </div>
            </div>
        </div>
    `;
    banner.classList.remove('hidden');
}

function handleDashboardMessage(message) {
    console.log('Dashboard message:', message);

    // Parse protocol message
    if (!message.version || !message.type || !message.payload) {
        console.warn('Invalid protocol message format:', message);
        return;
    }

    // Validate protocol version (major version must match)
    const expectedMajor = PROTOCOL_VERSION.split('.')[0];
    const receivedMajor = message.version.split('.')[0];
    if (expectedMajor !== receivedMajor) {
        console.error(`Protocol version mismatch: expected ${PROTOCOL_VERSION}, got ${message.version}`);
        return;
    }

    // Handle message based on type
    switch (message.type) {
        case 'error':
            // Dashboard sent an error
            console.error(`Dashboard error [${message.payload.code}]: ${message.payload.message}`);
            if (message.payload.details) {
                console.error('  Details:', message.payload.details);
            }

            // Handle critical errors
            if (message.payload.code === 'unsupported_version') {
                showErrorBanner('Protocol version mismatch. Please refresh the page or upgrade.');
            } else if (message.payload.code === 'invalid_path') {
                showErrorBanner(`Invalid project path: ${message.payload.message}`);
            }
            break;
        case 'welcome':
            // Protocol v1.0 handshake response
            console.log('✓ Received welcome from Dashboard');
            if (message.payload.session_id) {
                console.log(`  Session ID: ${message.payload.session_id}`);
            }
            // Init message will follow
            break;
        case 'init':
            // Initial project list from Dashboard
            handleInitMessage(message.payload.projects);
            break;
        case 'project_online':
            // Project came online
            handleProjectOnline(message.payload.project);
            break;
        case 'project_offline':
            // Project went offline
            handleProjectOffline(message.payload.project_path);
            break;
        case 'ping':
            // Heartbeat ping from server - respond with pong
            console.log('♥ Received heartbeat ping');
            if (dashboardWebSocket && dashboardWebSocket.readyState === WebSocket.OPEN) {
                const pongMsg = {
                    version: PROTOCOL_VERSION,
                    type: 'pong',
                    payload: {},
                    timestamp: new Date().toISOString()
                };
                dashboardWebSocket.send(JSON.stringify(pongMsg));
            }
            break;
        case 'goodbye':
            // Dashboard is closing connection gracefully
            if (message.payload.reason) {
                console.log(`✓ Dashboard closing connection: ${message.payload.reason}`);
            } else {
                console.log('✓ Dashboard closing connection gracefully');
            }
            // Connection will close - this is expected, not an error
            break;
        case 'task_created':
            // A new task was created via CLI/MCP
            handleTaskCreated(message.payload);
            break;
        case 'task_updated':
            // A task was updated via CLI/MCP
            handleTaskUpdated(message.payload);
            break;
        case 'task_deleted':
            // A task was deleted via CLI/MCP
            handleTaskDeleted(message.payload);
            break;
        case 'event_created':
            // A new event was created via CLI/MCP
            handleEventCreated(message.payload);
            break;
        default:
            console.warn('Unknown message type:', message.type);
    }
}

function handleInitMessage(projects) {
    console.log('Received initial project list:', projects);

    // Clear online projects map
    onlineProjects.clear();

    // Add all online projects to map
    projects.forEach(project => {
        onlineProjects.set(project.path, project);
        // Also add to storage if not already there
        addProjectToStorage(project);
    });

    // Render tabs
    renderProjectTabs();
}

function handleProjectOnline(project) {
    console.log('Project came online:', project);

    // Add to online projects map
    onlineProjects.set(project.path, project);

    // Add to storage if not already there
    addProjectToStorage(project);

    // Re-render tabs
    renderProjectTabs();
}

function handleProjectOffline(projectPath) {
    console.log('Project went offline:', projectPath);

    // Remove from online projects map
    onlineProjects.delete(projectPath);

    // Re-render tabs (project stays in storage, just shown as offline)
    renderProjectTabs();
}

// ============================================================================
// Database Operation Message Handlers
// ============================================================================

function handleTaskCreated(payload) {
    console.log('✨ Task created via CLI/MCP:', payload);

    // Only reload if this is for the current project
    const projects = JSON.parse(localStorage.getItem('intent-engine-projects') || '[]');
        const currentProject = projects.find(p => p.is_online);
        const currentProjectPath = currentProject?.path
    
    if (payload.project_path && currentProjectPath && payload.project_path !== currentProjectPath) {
          console.log('  Ignoring - different project');
          return;
      }
    

    // Reload task list to show the new task
    loadTasks();

    // Show brief notification
    showNotification('Task created', 'success');
}

function handleTaskUpdated(payload) {
    console.log('📝 Task updated via CLI/MCP:', payload);

    // Only reload if this is for the current project
    const projects = JSON.parse(localStorage.getItem('intent-engine-projects') || '[]');
        const currentProject = projects.find(p => p.is_online);
        const currentProjectPath = currentProject?.path
    
    if (payload.project_path && currentProjectPath && payload.project_path !== currentProjectPath) {
          console.log('  Ignoring - different project');
          return;
      }
    

    // Reload task list to show the updated task
    loadTasks();

    // If the updated task is the current task, reload current task details
    const updatedTaskId = payload.affected_ids?.[0];
    if (updatedTaskId && currentTaskId === updatedTaskId) {
        loadCurrentTask();
    }

    // Show brief notification
    showNotification('Task updated', 'info');
}

function handleTaskDeleted(payload) {
    console.log('🗑️  Task deleted via CLI/MCP:', payload);

    // Only reload if this is for the current project
    const projects = JSON.parse(localStorage.getItem('intent-engine-projects') || '[]');
        const currentProject = projects.find(p => p.is_online);
        const currentProjectPath = currentProject?.path
    
    if (payload.project_path && currentProjectPath && payload.project_path !== currentProjectPath) {
          console.log('  Ignoring - different project');
          return;
      }
    

    // Reload task list to remove the deleted task
    loadTasks();

    // If the deleted task is the current task, clear current task
    const deletedTaskId = payload.affected_ids?.[0];
    if (deletedTaskId && currentTaskId === deletedTaskId) {
        currentTaskId = null;
        currentTask = null;
        loadCurrentTask();
    }

    // Show brief notification
    showNotification('Task deleted', 'warning');
}

function handleEventCreated(payload) {
    console.log('📌 Event created via CLI/MCP:', payload);

    // Only reload if this is for the current project
    const projects = JSON.parse(localStorage.getItem('intent-engine-projects') || '[]');
        const currentProject = projects.find(p => p.is_online);
        const currentProjectPath = currentProject?.path
    
    if (payload.project_path && currentProjectPath && payload.project_path !== currentProjectPath) {
          console.log('  Ignoring - different project');
          return;
      }
    

    // If event is for the current task, reload task details (which includes events)
    if (payload.data?.task_id && currentTaskId === payload.data.task_id) {
        loadCurrentTask();
    }

    // Show brief notification
    showNotification('Event added', 'info');
}

function showNotification(message, type = 'info') {
    // Simple console notification for now
    // TODO: Implement visual toast notifications in the UI
    console.log(`[${type.toUpperCase()}] ${message}`);
}

// Render project tabs from storage + online state
function renderProjectTabs() {
    const container = document.getElementById('project-tabs');
    const storedProjects = loadProjectsFromStorage();

    if (storedProjects.length === 0 && onlineProjects.size === 0) {
        container.innerHTML = '<div class="text-xs font-mono text-slate-500 py-3">NO_PROJECTS_FOUND</div>';
        return;
    }

    // Helper function to render tabs
    const renderTabs = (currentProjectPath = '') => {
        const tabsHTML = storedProjects.map(project => {
            // Check if project is online based on is_online field (Dashboard running)
            const onlineProject = onlineProjects.get(project.path);
            const isOnline = onlineProject ? onlineProject.is_online : false;
            const isActive = project.path === currentProjectPath;
            const activeClass = isActive
                ? 'bg-neon-blue text-black font-bold shadow-neon-blue'
                : 'bg-sci-panel border border-sci-border text-slate-400 hover:text-white hover:border-white';

            // Status indicator: green for online, gray for offline
            const statusIndicator = isOnline
                ? '<span class="ml-1 text-neon-green animate-pulse" title="ONLINE">●</span>'
                : '<span class="ml-1 text-slate-600" title="OFFLINE">●</span>';

            // Delete button (X) for offline projects - shows on hover
            const deleteButton = !isOnline
                ? `<span class="ml-2 text-red-500 hover:text-red-300 cursor-pointer opacity-0 group-hover:opacity-100 transition-opacity" onclick="deleteProject('${escapeHtml(project.path)}'); event.stopPropagation();" title="DELETE">×</span>`
                : '';

            // Allow offline projects to be clicked (for read-only access)
            const clickHandler = isActive
                ? 'onclick="return false;"'
                : `onclick="switchProject('${escapeHtml(project.path)}', ${isOnline}); return false;"`;

            return `
                <a href="#" ${clickHandler} class="group px-4 py-2 text-xs font-mono transition-all whitespace-nowrap ${activeClass} flex items-center gap-1">
                    ${project.name.toUpperCase()}
                    ${statusIndicator}
                    ${deleteButton}
                </a>
            `;
        }).join('');

        container.innerHTML = tabsHTML;
    };

    // Try to get current project info, but gracefully degrade if it fails
    fetch('/api/info')
        .then(response => {
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}`);
            }
            return response.json();
        })
        .then(infoData => {
            const currentProjectPath = infoData.path || '';
            renderTabs(currentProjectPath);
        })
        .catch(error => {
            console.warn('Cannot fetch current project info, using cached data:', error);
            // Gracefully degrade: render tabs without knowing which is current
            renderTabs('');
        });
}

// Delete project from storage and UI
window.deleteProject = function(projectPath) {
    if (confirm(`Delete project tab for:\n${projectPath}\n\nThis will remove it from your browser storage.`)) {
        removeProjectFromStorage(projectPath);
        renderProjectTabs();
        showNotification('PROJECT_TAB_DELETED', 'success');
    }
};

// Load project info
async function loadProjectInfo() {
    try {
        const response = await fetch('/api/info');
        const data = await response.json();
        document.getElementById('project-name').textContent = (data.name || 'UNKNOWN_PROJECT').toUpperCase();
    } catch (e) {
        console.error('Failed to load project info:', e);
    }
}

// Load tasks with optional filter
async function loadTasks(status = null) {
    const container = document.getElementById('task-list-items');
    container.innerHTML = '<div class="text-center text-slate-600 font-mono text-xs py-4 animate-pulse">SCANNING_DATABASE...</div>';

    try {
        let url = '/api/tasks';
        if (status && status !== 'all') {
            url += `?status=${status}`;
        }

        const response = await fetch(url);
        const result = await response.json();
        const tasks = result.data || [];

        if (tasks.length === 0) {
            container.innerHTML = '<div class="text-center text-slate-600 font-mono text-xs py-8">NO_TASKS_DETECTED</div>';
            return;
        }

        container.innerHTML = tasks.map(task => renderTaskCard(task)).join('');
    } catch (e) {
        console.error('Failed to load tasks:', e);
        container.innerHTML = '<div class="text-center text-neon-red font-mono text-xs py-4">CONNECTION_FAILURE</div>';
    }
}

// Render task card
function renderTaskCard(task) {
    const statusClass = `status-${task.status}`;
    const priorityLabel = getPriorityLabel(task.priority);
    const isActive = task.id === currentTaskId ? 'active' : '';

    // Priority colors for border/accent
    let priorityColor = 'border-sci-border';
    if (task.priority === 1) priorityColor = 'border-neon-red';
    if (task.priority === 2) priorityColor = 'border-orange-500';

    return `
        <div class="task-card ${isActive} p-3 cursor-pointer mb-2 relative overflow-hidden group"
             onclick="loadTaskDetail(${task.id})">
            <div class="flex items-start justify-between mb-1">
                <span class="font-mono text-[10px] text-slate-500">ID::${task.id.toString().padStart(4, '0')}</span>
                <span class="status-badge ${statusClass}">${task.status}</span>
            </div>
            <h3 class="font-body font-semibold text-slate-200 text-sm leading-tight group-hover:text-neon-blue transition-colors">${escapeHtml(task.name)}</h3>
            
            <div class="flex items-center justify-between mt-2">
                ${task.parent_id ? `<span class="font-mono text-[10px] text-slate-600">PARENT::${task.parent_id}</span>` : '<span></span>'}
                ${priorityLabel ? `<span class="font-mono text-[10px] ${getPriorityColorClass(task.priority)}">[${priorityLabel.toUpperCase()}]</span>` : ''}
            </div>
        </div>
    `;
}

function getPriorityColorClass(priority) {
    if (priority === 1) return 'text-neon-red animate-pulse';
    if (priority === 2) return 'text-orange-500';
    if (priority === 3) return 'text-yellow-500';
    return 'text-slate-500';
}

// Get priority label
function getPriorityLabel(priority) {
    if (!priority) return null;
    const labels = { 1: 'Critical', 2: 'High', 3: 'Medium', 4: 'Low' };
    return labels[priority] || null;
}

// Load task detail
async function loadTaskDetail(taskId) {
    currentTaskId = taskId;
    const container = document.getElementById('task-detail-container');
    container.innerHTML = `
        <div class="h-full flex flex-col items-center justify-center text-slate-600">
            <div class="w-16 h-16 border-t-2 border-neon-blue rounded-full animate-spin mb-4"></div>
            <p class="font-mono text-xs text-neon-blue animate-pulse">ACCESSING_SECURE_DATA...</p>
        </div>
    `;

    try {
        const response = await fetch(`/api/tasks/${taskId}`);
        const result = await response.json();
        currentTask = result.data;

        // Render task detail
        container.innerHTML = renderTaskDetail(currentTask);

        // Load events for this task
        await loadEvents(taskId);

        // Update active state in task list
        document.querySelectorAll('.task-card').forEach(card => {
            card.classList.remove('active');
        });
        const activeCard = document.querySelector(`[onclick="loadTaskDetail(${taskId})"]`);
        if (activeCard) activeCard.classList.add('active');

    } catch (e) {
        console.error('Failed to load task detail:', e);
        container.innerHTML = '<div class="text-center text-neon-red font-mono py-20">DATA_CORRUPTION_DETECTED</div>';
    }
}

// Render task detail
function renderTaskDetail(task) {
    const statusClass = `status-${task.status}`;
    const priorityLabel = getPriorityLabel(task.priority);
    const spec = task.spec ? renderMarkdown(task.spec) : '<p class="text-slate-600 font-mono italic">// NO_DATA_AVAILABLE</p>';

    return `
        <div class="max-w-5xl mx-auto pb-20">
            <!-- Header Panel -->
            <div class="holo-panel p-6 mb-6 rounded-sm border-l-4 border-l-neon-blue">
                <div class="flex items-center justify-between mb-4">
                    <div class="flex items-center gap-4">
                        <span class="font-mono text-xs text-neon-blue border border-neon-blue px-2 py-1">ID::${task.id.toString().padStart(4, '0')}</span>
                        <span class="status-badge ${statusClass} text-sm">${task.status}</span>
                    </div>
                    <div class="flex items-center gap-2">
                        ${priorityLabel ? `<span class="font-mono text-xs px-2 py-1 border ${getPriorityBorderClass(task.priority)} ${getPriorityColorClass(task.priority)}">PRIORITY::${priorityLabel.toUpperCase()}</span>` : ''}
                    </div>
                </div>
                
                <h1 class="text-3xl md:text-4xl font-display font-bold text-white mb-4 text-shadow-neon">${escapeHtml(task.name)}</h1>
                
                <!-- Temporal Data Bar -->
                <div class="flex flex-wrap items-center gap-6 border-t border-sci-border pt-4 mt-4">
                    <div class="flex items-center gap-2">
                        <svg class="w-4 h-4 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
                        <span class="font-mono text-[10px] text-slate-500 uppercase tracking-wider">CREATED:</span>
                        <span class="font-mono text-xs text-neon-blue">${formatDate(task.first_todo_at)}</span>
                    </div>
                    ${task.first_doing_at ? `
                        <div class="flex items-center gap-2">
                            <svg class="w-4 h-4 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11-7-7z"></path></svg>
                            <span class="font-mono text-[10px] text-slate-500 uppercase tracking-wider">ACTIVATED:</span>
                            <span class="font-mono text-xs text-yellow-400">${formatDate(task.first_doing_at)}</span>
                        </div>
                    ` : ''}
                    ${task.first_done_at ? `
                        <div class="flex items-center gap-2">
                            <svg class="w-4 h-4 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>
                            <span class="font-mono text-[10px] text-slate-500 uppercase tracking-wider">COMPLETED:</span>
                            <span class="font-mono text-xs text-neon-green">${formatDate(task.first_done_at)}</span>
                        </div>
                    ` : ''}
                    ${task.parent_id ? `
                        <div class="flex items-center gap-2 ml-auto">
                            <span class="text-slate-500 font-mono text-[10px] uppercase tracking-wider">LINKED_PARENT:</span>
                            <a href="#" onclick="loadTaskDetail(${task.parent_id}); return false;" class="text-neon-purple hover:text-white font-mono text-xs transition-colors">#${task.parent_id}</a>
                        </div>
                    ` : ''}
                </div>
            </div>

            <!-- Command Protocols (Actions) -->
            ${isCurrentProjectOffline ? `
                <div class="bg-amber-900/20 border border-amber-600/50 rounded px-4 py-3 mb-8">
                    <p class="font-mono text-xs text-amber-400">⚠️ READ-ONLY MODE: All editing operations are disabled while project is offline.</p>
                </div>
            ` : `
                <div class="grid grid-cols-2 md:grid-cols-4 gap-3 mb-8">
                    ${task.status === 'todo' ? `
                        <button onclick="startTask(${task.id})" class="col-span-2 py-3 bg-neon-blue/10 border border-neon-blue text-neon-blue hover:bg-neon-blue hover:text-black font-display font-bold tracking-wider transition-all uppercase">
                            ▶ Initiate_Sequence
                        </button>
                    ` : ''}
                    ${task.status === 'doing' ? `
                        <button onclick="doneTask()" class="col-span-2 py-3 bg-neon-green/10 border border-neon-green text-neon-green hover:bg-neon-green hover:text-black font-display font-bold tracking-wider transition-all uppercase">
                            ✓ Mission_Complete
                        </button>
                        <button onclick="openSpawnSubtaskModal(${task.id})" class="py-3 bg-neon-purple/10 border border-neon-purple text-neon-purple hover:bg-neon-purple hover:text-white font-mono text-xs font-bold tracking-wider transition-all uppercase">
                            + Fork_Subprocess
                        </button>
                    ` : ''}
                    ${task.status !== 'doing' ? `
                        <button onclick="switchTask(${task.id})" class="py-3 bg-sci-panel border border-sci-border text-slate-300 hover:border-neon-blue hover:text-neon-blue font-mono text-xs font-bold tracking-wider transition-all uppercase">
                            ⇄ Switch_Focus
                        </button>
                    ` : ''}
                    <button onclick="openEditTaskModal(${task.id})" class="py-3 bg-sci-panel border border-sci-border text-yellow-500 hover:border-yellow-500 hover:text-black hover:bg-yellow-500 font-mono text-xs font-bold tracking-wider transition-all uppercase">
                        ✏️ Modify
                    </button>
                    <button onclick="openAddEventModal(${task.id})" class="py-3 bg-sci-panel border border-sci-border text-slate-300 hover:border-white hover:text-white font-mono text-xs font-bold tracking-wider transition-all uppercase">
                        📝 Log_Entry
                    </button>
                    <button onclick="deleteTask(${task.id})" class="py-3 bg-sci-panel border border-sci-border text-neon-red hover:bg-neon-red hover:text-black font-mono text-xs font-bold tracking-wider transition-all uppercase">
                        🗑 Terminate
                    </button>
                </div>
            `}

            <!-- Main Data Display -->
            <div class="grid grid-cols-1 gap-6">
                <!-- Specs -->
                <div class="w-full">
                    <div class="flex items-center gap-2 mb-3 border-b border-sci-border pb-2">
                        <svg class="w-5 h-5 text-neon-blue" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path></svg>
                        <h2 class="font-display text-lg text-white tracking-wider">MISSION_PARAMETERS</h2>
                    </div>
                    <div class="prose prose-invert max-w-none bg-sci-panel/30 p-6 rounded border border-sci-border/50 min-h-[200px]">
                        ${spec}
                    </div>
                </div>
            </div>
        </div>
    `;
}

function getPriorityBorderClass(priority) {
    if (priority === 1) return 'border-neon-red';
    if (priority === 2) return 'border-orange-500';
    if (priority === 3) return 'border-yellow-500';
    return 'border-slate-500';
}

// Load events for a task
async function loadEvents(taskId) {
    const container = document.getElementById('event-list-container');
    container.innerHTML = '<div class="text-center text-slate-600 font-mono text-xs py-4">DOWNLOADING_LOGS...</div>';

    try {
        const response = await fetch(`/api/tasks/${taskId}/events`);
        const result = await response.json();
        const events = result.data || [];

        if (events.length === 0) {
            container.innerHTML = '<div class="text-center text-slate-600 font-mono text-xs py-8">NO_LOGS_FOUND</div>';
            return;
        }

        container.innerHTML = events.map(event => renderEventCard(event)).join('');
    } catch (e) {
        console.error('Failed to load events:', e);
        container.innerHTML = '<div class="text-center text-neon-red font-mono text-xs py-4">LOG_RETRIEVAL_ERROR</div>';
    }
}

// Render event card
function renderEventCard(event) {
    const typeColors = {
        decision: 'border-neon-blue text-neon-blue',
        blocker: 'border-neon-red text-neon-red',
        milestone: 'border-neon-green text-neon-green',
        note: 'border-slate-500 text-slate-400'
    };
    const typeIcons = {
        decision: '💡',
        blocker: '🚫',
        milestone: '🎯',
        note: '📝'
    };

    const colorClass = typeColors[event.log_type] || typeColors.note;
    const icon = typeIcons[event.log_type] || '📝';

    return `
        <div class="group border-l-2 ${colorClass.split(' ')[0]} bg-sci-bg/50 p-4 mb-3 hover:bg-sci-panel transition-colors relative">
            <div class="flex items-center justify-between mb-2">
                <span class="font-mono text-xs font-bold uppercase tracking-wider ${colorClass.split(' ')[1]}">${icon} ${event.log_type}</span>
                <div class="flex items-center gap-3">
                    <span class="font-mono text-xs text-slate-500">${formatDate(event.timestamp || event.logged_at)}</span>
                    ${!isCurrentProjectOffline ? `
                        <div class="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                            <button onclick="openEditEventModal(${event.id}, ${event.task_id})"
                                class="text-yellow-500 hover:text-yellow-300 transition-colors"
                                title="Edit Event">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                        d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
                                </svg>
                            </button>
                            <button onclick="deleteEvent(${event.id}, ${event.task_id})"
                                class="text-neon-red hover:text-red-300 transition-colors"
                                title="Delete Event">
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                        d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"></path>
                                </svg>
                            </button>
                        </div>
                    ` : ''}
                </div>
            </div>
            <div class="prose prose-invert max-w-none text-base text-slate-300 leading-relaxed">
                ${renderMarkdown(event.discussion_data)}
            </div>
        </div>
    `;
}

// Task operations
async function startTask(taskId) {
    try {
        const response = await fetch(`/api/tasks/${taskId}/start`, { method: 'POST' });
        if (response.ok) {
            await loadTasks(currentFilter);
            await loadTaskDetail(taskId);
            await loadCurrentTask();
            showNotification('SEQUENCE_INITIATED', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'INITIATION_FAILED', 'error');
        }
    } catch (e) {
        console.error('Failed to start task:', e);
        showNotification('SYSTEM_ERROR', 'error');
    }
}

async function doneTask() {
    try {
        const response = await fetch('/api/tasks/done', { method: 'POST' });
        if (response.ok) {
            const result = await response.json();
            await loadTasks(currentFilter);
            if (result.data && result.data.id) {
                await loadTaskDetail(result.data.id);
            }
            await loadCurrentTask();
            showNotification('MISSION_COMPLETE', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'COMPLETION_FAILED', 'error');
        }
    } catch (e) {
        console.error('Failed to complete task:', e);
        showNotification('SYSTEM_ERROR', 'error');
    }
}

async function switchTask(taskId) {
    try {
        const response = await fetch(`/api/tasks/${taskId}/switch`, { method: 'POST' });
        if (response.ok) {
            await loadTasks(currentFilter);
            await loadTaskDetail(taskId);
            await loadCurrentTask();
            showNotification('FOCUS_SWITCHED', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'SWITCH_FAILED', 'error');
        }
    } catch (e) {
        console.error('Failed to switch task:', e);
        showNotification('SYSTEM_ERROR', 'error');
    }
}

async function deleteTask(taskId) {
    if (!confirm('WARNING: TERMINATING TASK DATA. THIS ACTION IS IRREVERSIBLE. PROCEED?')) {
        return;
    }

    try {
        const response = await fetch(`/api/tasks/${taskId}`, { method: 'DELETE' });
        if (response.ok) {
            await loadTasks(currentFilter);
            document.getElementById('task-detail-container').innerHTML = `
                <div class="h-full flex flex-col items-center justify-center text-neon-red">
                    <p class="font-display text-xl">TARGET_ELIMINATED</p>
                </div>
            `;
            showNotification('TASK_TERMINATED', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'TERMINATION_FAILED', 'error');
        }
    } catch (e) {
        console.error('Failed to delete task:', e);
        showNotification('SYSTEM_ERROR', 'error');
    }
}

async function loadCurrentTask() {
    try {
        const response = await fetch('/api/current-task');
        const result = await response.json();

        if (result.data && result.data.task) {
            currentTaskId = result.data.task.id;
        } else {
            currentTaskId = null;
        }
    } catch (e) {
        console.error('Failed to load current task:', e);
    }
}

// Refresh all data (for CLI/MCP sync)
async function refreshAllData() {
    try {
        // Show loading notification
        showNotification('REFRESHING_DATA...', 'info');

        // Reload tasks list
        await loadTasks(currentFilter);

        // Reload current task if one is selected
        if (currentTaskId) {
            await loadTaskDetail(currentTaskId);
        }

        // Reload current focus
        await loadCurrentTask();

        // Success notification
        showNotification('DATA_REFRESHED', 'success');
    } catch (e) {
        console.error('Failed to refresh data:', e);
        showNotification('REFRESH_FAILED', 'error');
    }
}

// Filter tasks
function filterTasks(status) {
    currentFilter = status;

    // Update button styles
    const buttons = {
        'all': document.getElementById('filter-all'),
        'todo': document.getElementById('filter-todo'),
        'doing': document.getElementById('filter-doing'),
        'done': document.getElementById('filter-done')
    };

    // Reset all
    Object.values(buttons).forEach(btn => {
        btn.className = 'flex-1 py-1 text-xs font-mono border border-sci-border text-slate-400 hover:text-white transition-colors';
    });

    // Set active
    const activeBtn = buttons[status];
    if (activeBtn) {
        activeBtn.className = 'flex-1 py-1 text-xs font-mono border border-neon-blue bg-neon-blue/10 text-neon-blue transition-colors';
    }

    loadTasks(status === 'all' ? null : status);
}

// Search
let searchTimeout;
function handleSearch(event) {
    clearTimeout(searchTimeout);
    const query = event.target.value.trim();

    if (!query) {
        loadTasks(currentFilter === 'all' ? null : currentFilter);
        return;
    }

    searchTimeout = setTimeout(async () => {
        const container = document.getElementById('task-list-items');
        container.innerHTML = '<div class="text-center text-neon-blue font-mono text-xs py-4 animate-pulse">SEARCHING_DATABASE...</div>';

        try {
            const response = await fetch(`/api/search?query=${encodeURIComponent(query)}`);

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            const result = await response.json();
            const results = result.data || [];

            if (results.length === 0) {
                container.innerHTML = '<div class="text-center text-slate-600 font-mono text-xs py-8">NO_MATCHES_FOUND</div>';
                return;
            }

            // Extract tasks from search results
            const tasks = results
                .filter(r => r.result_type === 'task')
                .map(r => r.task || r)
                .filter(t => t && t.id && t.name && t.status);

            if (tasks.length === 0) {
                container.innerHTML = '<div class="text-center text-slate-600 font-mono text-xs py-8">NO_TASKS_FOUND (EVENTS MATCHED)</div>';
                return;
            }

            container.innerHTML = tasks.map(task => renderTaskCard(task)).join('');
        } catch (e) {
            console.error('Search failed:', e);
            container.innerHTML = '<div class="text-center text-neon-red font-mono text-xs py-4">SEARCH_ERROR</div>';
        }
    }, 300);
}

// Modal functions
function openNewTaskModal() {
    document.getElementById('new-task-modal').classList.remove('hidden');
}

function closeNewTaskModal() {
    document.getElementById('new-task-modal').classList.add('hidden');
    document.getElementById('new-task-form').reset();
}

async function createTask(event) {
    event.preventDefault();
    const form = event.target;
    const formData = new FormData(form);

    const data = {
        name: formData.get('name'),
        spec: formData.get('spec') || null,
        priority: formData.get('priority') ? parseInt(formData.get('priority')) : null,
        parent_id: formData.get('parent_id') ? parseInt(formData.get('parent_id')) : null
    };

    try {
        const response = await fetch('/api/tasks', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(data)
        });

        if (response.ok) {
            const result = await response.json();
            closeNewTaskModal();
            await loadTasks(currentFilter);
            await loadTaskDetail(result.data.id);
            showNotification('TASK_INITIALIZED', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'INITIALIZATION_FAILED', 'error');
        }
    } catch (e) {
        console.error('Failed to create task:', e);
        showNotification('SYSTEM_ERROR', 'error');
    }
}

// Edit task modal functions
async function openEditTaskModal(taskId) {
    try {
        const response = await fetch(`/api/tasks/${taskId}`);
        if (!response.ok) {
            showNotification('FAILED_TO_LOAD_TASK', 'error');
            return;
        }
        const result = await response.json();
        const task = result.data;  // Extract actual task from wrapper

        document.getElementById('edit-task-id').value = task.id;
        document.getElementById('edit-task-name').value = task.name;
        document.getElementById('edit-task-spec').value = task.spec || '';
        document.getElementById('edit-task-priority').value = task.priority || '';
        document.getElementById('edit-task-status').value = task.status;
        document.getElementById('edit-task-modal').classList.remove('hidden');
    } catch (e) {
        console.error('Failed to load task:', e);
        showNotification('SYSTEM_ERROR', 'error');
    }
}

function closeEditTaskModal() {
    document.getElementById('edit-task-modal').classList.add('hidden');
    document.getElementById('edit-task-form').reset();
}

async function updateTask(event) {
    event.preventDefault();
    const form = event.target;
    const formData = new FormData(form);
    const taskId = parseInt(formData.get('task_id'));

    const data = {
        name: formData.get('name'),
        spec: formData.get('spec') || null,
        priority: formData.get('priority') ? parseInt(formData.get('priority')) : null,
        status: formData.get('status')
    };

    try {
        const response = await fetch(`/api/tasks/${taskId}`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(data)
        });

        if (response.ok) {
            closeEditTaskModal();
            await loadTasks(currentFilter);
            await loadTaskDetail(taskId);
            showNotification('TASK_UPDATED', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'UPDATE_FAILED', 'error');
        }
    } catch (e) {
        console.error('Failed to update task:', e);
        showNotification('SYSTEM_ERROR', 'error');
    }
}

function openAddEventModal(taskId) {
    document.getElementById('event-task-id').value = taskId;
    document.getElementById('add-event-modal').classList.remove('hidden');
}

function closeAddEventModal() {
    document.getElementById('add-event-modal').classList.add('hidden');
    document.getElementById('add-event-form').reset();
}

async function addEvent(event) {
    event.preventDefault();
    const form = event.target;
    const formData = new FormData(form);
    const taskId = parseInt(formData.get('task_id'));

    const data = {
        type: formData.get('type'),
        data: formData.get('data')
    };

    try {
        const response = await fetch(`/api/tasks/${taskId}/events`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(data)
        });

        if (response.ok) {
            closeAddEventModal();
            await loadEvents(taskId);
            showNotification('LOG_COMMITTED', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'LOG_FAILURE', 'error');
        }
    } catch (e) {
        console.error('Failed to add event:', e);
        showNotification('SYSTEM_ERROR', 'error');
    }
}

// Edit event modal functions
async function openEditEventModal(eventId, taskId) {
    try {
        const response = await fetch(`/api/tasks/${taskId}/events`);
        if (!response.ok) {
            showNotification('FAILED_TO_LOAD_EVENTS', 'error');
            return;
        }
        const result = await response.json();
        const events = result.data;  // Extract events array from wrapper
        const event = events.find(e => e.id === eventId);

        if (!event) {
            showNotification('EVENT_NOT_FOUND', 'error');
            return;
        }

        document.getElementById('edit-event-id').value = event.id;
        document.getElementById('edit-event-task-id').value = event.task_id;
        document.getElementById('edit-event-type').value = event.log_type;
        document.getElementById('edit-event-data').value = event.discussion_data;
        document.getElementById('edit-event-modal').classList.remove('hidden');
    } catch (e) {
        console.error('Failed to load event:', e);
        showNotification('SYSTEM_ERROR', 'error');
    }
}

function closeEditEventModal() {
    document.getElementById('edit-event-modal').classList.add('hidden');
    document.getElementById('edit-event-form').reset();
}

async function updateEvent(event) {
    event.preventDefault();
    const form = event.target;
    const formData = new FormData(form);
    const eventId = parseInt(formData.get('event_id'));
    const taskId = parseInt(formData.get('task_id'));

    const data = {
        type: formData.get('type'),
        data: formData.get('data')
    };

    try {
        const response = await fetch(`/api/tasks/${taskId}/events/${eventId}`, {
            method: 'PATCH',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(data)
        });

        if (response.ok) {
            closeEditEventModal();
            await loadEvents(taskId);
            showNotification('EVENT_UPDATED', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'UPDATE_FAILED', 'error');
        }
    } catch (e) {
        console.error('Failed to update event:', e);
        showNotification('SYSTEM_ERROR', 'error');
    }
}

async function deleteEvent(eventId, taskId) {
    if (!confirm('WARNING: DELETE EVENT DATA. THIS ACTION IS IRREVERSIBLE. PROCEED?')) {
        return;
    }

    try {
        const response = await fetch(`/api/tasks/${taskId}/events/${eventId}`, {
            method: 'DELETE'
        });

        if (response.ok || response.status === 204) {
            await loadEvents(taskId);
            showNotification('EVENT_DELETED', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'DELETE_FAILED', 'error');
        }
    } catch (e) {
        console.error('Failed to delete event:', e);
        showNotification('SYSTEM_ERROR', 'error');
    }
}

function openSpawnSubtaskModal(parentId) {
    const name = prompt('ENTER_SUBTASK_DESIGNATION:');
    if (!name) return;

    const spec = prompt('ENTER_PARAMETERS [MARKDOWN]:');

    spawnSubtask(parentId, name, spec);
}

async function spawnSubtask(parentId, name, spec) {
    const data = {
        name: name,
        spec: spec || null
    };

    try {
        const response = await fetch(`/api/tasks/${parentId}/spawn-subtask`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(data)
        });

        if (response.ok) {
            const result = await response.json();
            await loadTasks(currentFilter);
            await loadTaskDetail(result.data.subtask.id);
            await loadCurrentTask();
            showNotification('SUBPROCESS_SPAWNED', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'SPAWN_FAILED', 'error');
        }
    } catch (e) {
        console.error('Failed to spawn subtask:', e);
        showNotification('SYSTEM_ERROR', 'error');
    }
}

// Utility functions
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function formatDate(dateStr) {
    if (!dateStr) return 'N/A';
    const date = new Date(dateStr);
    return date.toLocaleString('en-US', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        hour12: false
    }).replace(',', '');
}

function showNotification(message, type = 'info') {
    // In a real app, we'd use a toast. For now, console log or alert for errors.
    console.log(`[SYSTEM_MSG] ${type.toUpperCase()}: ${message}`);
    if (type === 'error') alert(`SYSTEM_ERROR: ${message}`);
}

// Switch to a different project
async function switchProject(projectPath, isOnline = true) {
    try {
        // Set offline mode state
        isCurrentProjectOffline = !isOnline;

        showNotification('REROUTING_SYSTEM...', 'info');

        const response = await fetch('/api/switch-project', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ project_path: projectPath })
        });

        if (!response.ok) {
            const error = await response.json();
            showNotification(error.message || 'REROUTE_FAILED', 'error');
            return;
        }

        const result = await response.json();
        const newProjectName = result.data.project_name;

        // Show different notification based on online/offline mode
        if (isOnline) {
            showNotification(`CONNECTED: ${newProjectName.toUpperCase()}`, 'success');
        } else {
            showNotification(`READ_ONLY MODE: ${newProjectName.toUpperCase()} (OFFLINE)`, 'info');
        }

        // Reload all data for the new project
        await loadProjectInfo();
        renderProjectTabs();
        await loadTasks(currentFilter);

        // Clear task detail view
        document.getElementById('task-detail-container').innerHTML = `
            <div class="h-full flex flex-col items-center justify-center text-slate-600">
                <div class="w-24 h-24 border border-sci-border rounded-full flex items-center justify-center mb-4 relative">
                    <div class="absolute inset-0 border-t border-neon-blue rounded-full animate-spin"></div>
                    <svg class="w-10 h-10 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"></path></svg>
                </div>
                <h2 class="font-display text-xl tracking-widest">AWAITING INPUT</h2>
                <p class="font-mono text-sm mt-2 text-neon-blue opacity-50">SELECT_TASK_MODULE</p>
            </div>
        `;

        // Clear event history
        document.getElementById('event-list-container').innerHTML = `
            <div class="text-center text-slate-600 font-mono text-xs py-8">
                NO_DATA_STREAM
            </div>
        `;

        // Reset current task
        currentTaskId = null;
        currentTask = null;

        // Update offline mode UI
        updateOfflineModeUI();

    } catch (e) {
        console.error('Failed to switch project:', e);
        showNotification('SYSTEM_ERROR', 'error');
    }
}

// Update UI based on offline mode state
function updateOfflineModeUI() {
    const banner = document.getElementById('offline-mode-banner');
    const newTaskBtn = document.querySelector('button[onclick="openNewTaskModal()"]');
    const currentFocusBtn = document.querySelector('button[onclick="loadCurrentTask()"]');

    if (isCurrentProjectOffline) {
        // Show offline banner
        banner.classList.remove('hidden');

        // Disable edit buttons
        if (newTaskBtn) {
            newTaskBtn.disabled = true;
            newTaskBtn.classList.add('opacity-50', 'cursor-not-allowed');
            newTaskBtn.classList.remove('hover:bg-white', 'hover:shadow-neon-blue');
        }
        if (currentFocusBtn) {
            currentFocusBtn.disabled = true;
            currentFocusBtn.classList.add('opacity-50', 'cursor-not-allowed');
            currentFocusBtn.classList.remove('hover:bg-neon-purple', 'hover:text-white');
        }
    } else {
        // Hide offline banner
        banner.classList.add('hidden');

        // Enable edit buttons
        if (newTaskBtn) {
            newTaskBtn.disabled = false;
            newTaskBtn.classList.remove('opacity-50', 'cursor-not-allowed');
            newTaskBtn.classList.add('hover:bg-white', 'hover:shadow-neon-blue');
        }
        if (currentFocusBtn) {
            currentFocusBtn.disabled = false;
            currentFocusBtn.classList.remove('opacity-50', 'cursor-not-allowed');
            currentFocusBtn.classList.add('hover:bg-neon-purple', 'hover:text-white');
        }
    }
}
