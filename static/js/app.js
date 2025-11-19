// Global state
let currentFilter = 'all';
let currentTaskId = null;
let currentTask = null;

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

    // Load project tabs
    await loadProjectTabs();

    // Load project info
    await loadProjectInfo();

    // Load tasks
    await loadTasks();

    // Load current task if exists
    await loadCurrentTask();

    // Refresh project tabs every 30 seconds
    setInterval(loadProjectTabs, 30000);
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
                <button onclick="openAddEventModal(${task.id})" class="py-3 bg-sci-panel border border-sci-border text-slate-300 hover:border-white hover:text-white font-mono text-xs font-bold tracking-wider transition-all uppercase">
                    📝 Log_Entry
                </button>
                <button onclick="deleteTask(${task.id})" class="py-3 bg-sci-panel border border-sci-border text-neon-red hover:bg-neon-red hover:text-black font-mono text-xs font-bold tracking-wider transition-all uppercase">
                    🗑 Terminate
                </button>
            </div>

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
        <div class="border-l-2 ${colorClass.split(' ')[0]} bg-sci-bg/50 p-4 mb-3 hover:bg-sci-panel transition-colors">
            <div class="flex items-center justify-between mb-2">
                <span class="font-mono text-xs font-bold uppercase tracking-wider ${colorClass.split(' ')[1]}">${icon} ${event.log_type}</span>
                <span class="font-mono text-xs text-slate-500">${formatDate(event.logged_at)}</span>
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

// Load project tabs
async function loadProjectTabs() {
    try {
        const response = await fetch('/api/projects');
        const result = await response.json();

        if (!result.data || result.data.length === 0) {
            document.getElementById('project-tabs').innerHTML = '<div class="text-xs font-mono text-slate-500 py-3">NO_PROJECTS_FOUND</div>';
            return;
        }

        // Get current project info
        const infoResponse = await fetch('/api/info');
        const infoData = await infoResponse.json();
        const currentProjectPath = infoData.path || '';

        const tabsHTML = result.data.map(project => {
            const isActive = project.path === currentProjectPath;
            const activeClass = isActive
                ? 'bg-neon-blue text-black font-bold shadow-neon-blue'
                : 'bg-sci-panel border border-sci-border text-slate-400 hover:text-white hover:border-white';

            // MCP connection status indicator
            const mcpConnected = project.mcp_connected || false;
            const mcpIndicator = mcpConnected
                ? '<span class="ml-1 text-neon-green" title="AGENT_ONLINE">●</span>'
                : '<span class="ml-1 text-slate-600" title="AGENT_OFFLINE">○</span>';

            const clickHandler = isActive
                ? 'onclick="return false;"'
                : `onclick="switchProject('${escapeHtml(project.path)}'); return false;"`;

            return `
                <a href="#" ${clickHandler} class="px-4 py-2 text-xs font-mono transition-all whitespace-nowrap ${activeClass} flex items-center gap-2">
                    ${project.name.toUpperCase()}
                    ${mcpIndicator}
                </a>
            `;
        }).join('');

        document.getElementById('project-tabs').innerHTML = tabsHTML;
    } catch (error) {
        console.error('Failed to load project tabs:', error);
        document.getElementById('project-tabs').innerHTML = '<div class="text-xs font-mono text-neon-red py-3">LOAD_ERROR</div>';
    }
}

// Switch to a different project
async function switchProject(projectPath) {
    try {
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

        showNotification(`CONNECTED: ${newProjectName.toUpperCase()}`, 'success');

        // Reload all data for the new project
        await loadProjectInfo();
        await loadProjectTabs();
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

    } catch (e) {
        console.error('Failed to switch project:', e);
        showNotification('SYSTEM_ERROR', 'error');
    }
}
