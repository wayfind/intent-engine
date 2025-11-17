// Global state
let currentFilter = 'all';
let currentTaskId = null;
let currentTask = null;

// Initialize on page load
document.addEventListener('DOMContentLoaded', async () => {
    console.log('Intent-Engine Dashboard initializing...');

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
        return '<p>Error rendering markdown</p>';
    }
};

// Load project info
async function loadProjectInfo() {
    try {
        const response = await fetch('/api/info');
        const data = await response.json();
        document.getElementById('project-name').textContent = data.name || 'Unknown Project';
    } catch (e) {
        console.error('Failed to load project info:', e);
    }
}

// Load tasks with optional filter
async function loadTasks(status = null) {
    const container = document.getElementById('task-list-items');
    container.innerHTML = '<div class="text-center text-gray-500 py-4">Loading...</div>';

    try {
        let url = '/api/tasks';
        if (status && status !== 'all') {
            url += `?status=${status}`;
        }

        const response = await fetch(url);
        const result = await response.json();
        const tasks = result.data || [];

        if (tasks.length === 0) {
            container.innerHTML = '<div class="text-center text-gray-400 py-8 text-sm">No tasks found</div>';
            return;
        }

        container.innerHTML = tasks.map(task => renderTaskCard(task)).join('');
    } catch (e) {
        console.error('Failed to load tasks:', e);
        container.innerHTML = '<div class="text-center text-red-500 py-4">Failed to load tasks</div>';
    }
}

// Render task card
function renderTaskCard(task) {
    const statusClass = `status-${task.status}`;
    const priorityLabel = getPriorityLabel(task.priority);
    const isActive = task.id === currentTaskId ? 'active' : '';

    return `
        <div class="task-card ${isActive} border border-gray-200 rounded-lg p-3 cursor-pointer transition"
             onclick="loadTaskDetail(${task.id})">
            <div class="flex items-start justify-between mb-2">
                <span class="text-xs font-semibold text-gray-500">#${task.id}</span>
                <span class="text-xs px-2 py-1 rounded ${statusClass} font-medium">${task.status}</span>
            </div>
            <h3 class="text-sm font-medium text-gray-800 line-clamp-2">${escapeHtml(task.name)}</h3>
            ${task.parent_id ? `<p class="text-xs text-gray-500 mt-1">Parent: #${task.parent_id}</p>` : ''}
            ${priorityLabel ? `<span class="inline-block text-xs px-2 py-0.5 rounded mt-2 priority-${priorityLabel.toLowerCase()}">${priorityLabel}</span>` : ''}
        </div>
    `;
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
    container.innerHTML = '<div class="text-center text-gray-500 py-20">Loading...</div>';

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
        container.innerHTML = '<div class="text-center text-red-500 py-20">Failed to load task</div>';
    }
}

// Render task detail
function renderTaskDetail(task) {
    const statusClass = `status-${task.status}`;
    const priorityLabel = getPriorityLabel(task.priority);
    const spec = task.spec ? renderMarkdown(task.spec) : '<p class="text-gray-400 italic">No specification provided</p>';

    return `
        <div class="max-w-4xl mx-auto p-8">
            <!-- Header -->
            <div class="mb-6">
                <div class="flex items-center justify-between mb-4">
                    <span class="text-sm font-semibold text-gray-500">#${task.id}</span>
                    <div class="flex items-center space-x-2">
                        <span class="text-xs px-3 py-1 rounded ${statusClass} font-medium">${task.status}</span>
                        ${priorityLabel ? `<span class="text-xs px-3 py-1 rounded priority-${priorityLabel.toLowerCase()} font-medium">${priorityLabel}</span>` : ''}
                    </div>
                </div>
                <h1 class="text-3xl font-bold text-gray-900">${escapeHtml(task.name)}</h1>
                ${task.parent_id ? `<p class="text-sm text-gray-500 mt-2">Parent Task: <a href="#" onclick="loadTaskDetail(${task.parent_id}); return false;" class="text-indigo-600 hover:underline">#${task.parent_id}</a></p>` : ''}
            </div>

            <!-- Actions -->
            <div class="mb-6 flex flex-wrap gap-2">
                ${task.status === 'todo' ? `
                    <button onclick="startTask(${task.id})" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition">
                        ▶ Start Task
                    </button>
                ` : ''}
                ${task.status === 'doing' ? `
                    <button onclick="doneTask()" class="px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 transition">
                        ✓ Complete Task
                    </button>
                    <button onclick="openSpawnSubtaskModal(${task.id})" class="px-4 py-2 bg-purple-600 text-white rounded-lg hover:bg-purple-700 transition">
                        + Spawn Subtask
                    </button>
                ` : ''}
                ${task.status !== 'doing' ? `
                    <button onclick="switchTask(${task.id})" class="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition">
                        ⇄ Switch to This
                    </button>
                ` : ''}
                <button onclick="openAddEventModal(${task.id})" class="px-4 py-2 bg-gray-600 text-white rounded-lg hover:bg-gray-700 transition">
                    📝 Add Event
                </button>
                <button onclick="deleteTask(${task.id})" class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition">
                    🗑 Delete
                </button>
            </div>

            <!-- Specification -->
            <div class="mb-8">
                <h2 class="text-xl font-semibold text-gray-800 mb-4">Specification</h2>
                <div class="prose prose-slate bg-gray-50 p-6 rounded-lg border border-gray-200">
                    ${spec}
                </div>
            </div>

            <!-- Metadata -->
            <div class="mb-8 grid grid-cols-2 gap-4">
                <div class="bg-gray-50 p-4 rounded-lg border border-gray-200">
                    <p class="text-xs text-gray-500 mb-1">Created</p>
                    <p class="text-sm font-medium">${formatDate(task.first_todo_at)}</p>
                </div>
                ${task.first_doing_at ? `
                    <div class="bg-gray-50 p-4 rounded-lg border border-gray-200">
                        <p class="text-xs text-gray-500 mb-1">Started</p>
                        <p class="text-sm font-medium">${formatDate(task.first_doing_at)}</p>
                    </div>
                ` : ''}
                ${task.first_done_at ? `
                    <div class="bg-gray-50 p-4 rounded-lg border border-gray-200">
                        <p class="text-xs text-gray-500 mb-1">Completed</p>
                        <p class="text-sm font-medium">${formatDate(task.first_done_at)}</p>
                    </div>
                ` : ''}
            </div>
        </div>
    `;
}

// Load events for a task
async function loadEvents(taskId) {
    const container = document.getElementById('event-list-container');
    container.innerHTML = '<div class="text-center text-gray-400 py-4 text-sm">Loading events...</div>';

    try {
        const response = await fetch(`/api/tasks/${taskId}/events`);
        const result = await response.json();
        const events = result.data || [];

        if (events.length === 0) {
            container.innerHTML = '<div class="text-center text-gray-400 py-8 text-sm">No events yet</div>';
            return;
        }

        container.innerHTML = events.map(event => renderEventCard(event)).join('');
    } catch (e) {
        console.error('Failed to load events:', e);
        container.innerHTML = '<div class="text-center text-red-500 py-4 text-sm">Failed to load events</div>';
    }
}

// Render event card
function renderEventCard(event) {
    const typeColors = {
        decision: 'bg-blue-50 border-blue-200 text-blue-800',
        blocker: 'bg-red-50 border-red-200 text-red-800',
        milestone: 'bg-green-50 border-green-200 text-green-800',
        note: 'bg-gray-50 border-gray-200 text-gray-800'
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
        <div class="border ${colorClass} rounded-lg p-3 mb-3">
            <div class="flex items-center justify-between mb-2">
                <span class="text-xs font-semibold">${icon} ${event.log_type.toUpperCase()}</span>
                <span class="text-xs text-gray-500">${formatDate(event.logged_at)}</span>
            </div>
            <div class="text-sm prose prose-sm max-w-none">
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
            showNotification('Task started successfully', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'Failed to start task', 'error');
        }
    } catch (e) {
        console.error('Failed to start task:', e);
        showNotification('Failed to start task', 'error');
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
            showNotification('Task completed successfully', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'Failed to complete task', 'error');
        }
    } catch (e) {
        console.error('Failed to complete task:', e);
        showNotification('Failed to complete task', 'error');
    }
}

async function switchTask(taskId) {
    try {
        const response = await fetch(`/api/tasks/${taskId}/switch`, { method: 'POST' });
        if (response.ok) {
            await loadTasks(currentFilter);
            await loadTaskDetail(taskId);
            await loadCurrentTask();
            showNotification('Switched to task successfully', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'Failed to switch task', 'error');
        }
    } catch (e) {
        console.error('Failed to switch task:', e);
        showNotification('Failed to switch task', 'error');
    }
}

async function deleteTask(taskId) {
    if (!confirm('Are you sure you want to delete this task? This cannot be undone.')) {
        return;
    }

    try {
        const response = await fetch(`/api/tasks/${taskId}`, { method: 'DELETE' });
        if (response.ok) {
            await loadTasks(currentFilter);
            document.getElementById('task-detail-container').innerHTML = `
                <div class="max-w-4xl mx-auto p-8 text-center">
                    <p class="text-green-600 text-lg">Task deleted successfully</p>
                </div>
            `;
            showNotification('Task deleted successfully', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'Failed to delete task', 'error');
        }
    } catch (e) {
        console.error('Failed to delete task:', e);
        showNotification('Failed to delete task', 'error');
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

async function pickNextTask() {
    try {
        const response = await fetch('/api/pick-next');
        const result = await response.json();

        if (result.data && result.data.task) {
            const task = result.data.task;
            const reason = result.data.reason || 'Recommended next task';

            if (confirm(`Pick next task: #${task.id} "${task.name}"?\n\nReason: ${reason}`)) {
                await loadTaskDetail(task.id);
            }
        } else {
            showNotification('No tasks available to pick', 'info');
        }
    } catch (e) {
        console.error('Failed to pick next task:', e);
        showNotification('Failed to pick next task', 'error');
    }
}

// Filter tasks
function filterTasks(status) {
    currentFilter = status;

    // Update button styles
    document.querySelectorAll('[id^="filter-"]').forEach(btn => {
        btn.className = 'flex-1 px-3 py-2 text-sm font-medium bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200';
    });
    document.getElementById(`filter-${status}`).className = 'flex-1 px-3 py-2 text-sm font-medium bg-indigo-100 text-indigo-700 rounded-lg';

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
        container.innerHTML = '<div class="text-center text-gray-500 py-4">Searching...</div>';

        try {
            const response = await fetch(`/api/search?query=${encodeURIComponent(query)}`);

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }

            const result = await response.json();
            const results = result.data || [];

            if (results.length === 0) {
                container.innerHTML = '<div class="text-center text-gray-400 py-8 text-sm">No results found</div>';
                return;
            }

            // Extract tasks from search results - handle both old and new API format
            const tasks = results
                .filter(r => r.result_type === 'task')
                .map(r => {
                    // New format has task as a direct field in the result
                    if (r.task) {
                        return r.task;
                    }
                    // Old format might have task data directly in result
                    return r;
                })
                .filter(t => t && t.id && t.name && t.status); // Filter out invalid tasks

            if (tasks.length === 0) {
                container.innerHTML = '<div class="text-center text-gray-400 py-8 text-sm">No tasks found (events matched)</div>';
                return;
            }

            container.innerHTML = tasks.map(task => renderTaskCard(task)).join('');
        } catch (e) {
            console.error('Search failed:', e);
            container.innerHTML = '<div class="text-center text-red-500 py-4">Search failed</div>';
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
            showNotification('Task created successfully', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'Failed to create task', 'error');
        }
    } catch (e) {
        console.error('Failed to create task:', e);
        showNotification('Failed to create task', 'error');
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
            showNotification('Event added successfully', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'Failed to add event', 'error');
        }
    } catch (e) {
        console.error('Failed to add event:', e);
        showNotification('Failed to add event', 'error');
    }
}

function openSpawnSubtaskModal(parentId) {
    const name = prompt('Enter subtask name:');
    if (!name) return;

    const spec = prompt('Enter subtask specification (optional, Markdown):');

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
            showNotification('Subtask created and switched', 'success');
        } else {
            const error = await response.json();
            showNotification(error.message || 'Failed to spawn subtask', 'error');
        }
    } catch (e) {
        console.error('Failed to spawn subtask:', e);
        showNotification('Failed to spawn subtask', 'error');
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
    return date.toLocaleString();
}

function showNotification(message, type = 'info') {
    // Simple notification using alert for now
    // In production, use a proper notification library
    const colors = {
        success: '✓',
        error: '✗',
        info: 'ℹ'
    };
    console.log(`${colors[type]} ${message}`);

    // You could implement a toast notification here
    if (type === 'error') {
        alert(message);
    }
}

// Load project tabs
async function loadProjectTabs() {
    try {
        const response = await fetch('/api/projects');
        const result = await response.json();

        if (!result.data || result.data.length === 0) {
            document.getElementById('project-tabs').innerHTML = '<div class="text-sm text-gray-500 py-3">No projects found</div>';
            return;
        }

        const currentPort = window.location.port || '3030';

        const tabsHTML = result.data.map(project => {
            const isActive = project.port.toString() === currentPort;
            const activeClass = isActive ? 'border-b-2 border-indigo-600 text-indigo-600' : 'text-gray-600 hover:text-gray-800 hover:bg-gray-50';
            const indicator = isActive ? '<span class="ml-1">●</span>' : '';

            // MCP connection status indicator
            const mcpConnected = project.mcp_connected || false;
            const mcpAgent = project.mcp_agent || 'unknown';
            const mcpIndicator = mcpConnected
                ? `<span class="ml-1 text-green-500" title="Agent connected: ${mcpAgent}">🟢</span>`
                : '<span class="ml-1 text-gray-400" title="No agent connected">⚪</span>';

            const tooltipText = `${project.path}${mcpConnected ? '\n🟢 Agent: ' + mcpAgent : '\n⚪ No agent connected'}`;

            return `<a href="${project.url}" class="px-4 py-3 text-sm font-medium transition-colors ${activeClass} whitespace-nowrap" title="${tooltipText}">${project.name}${indicator}${mcpIndicator}</a>`;
        }).join('');

        document.getElementById('project-tabs').innerHTML = tabsHTML;
    } catch (error) {
        console.error('Failed to load project tabs:', error);
        document.getElementById('project-tabs').innerHTML = '<div class="text-sm text-red-500 py-3">Failed to load projects</div>';
    }
}
