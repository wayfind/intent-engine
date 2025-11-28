import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface Task {
    id: number
    name: string
    spec: string | null
    status: 'todo' | 'doing' | 'done'
    priority: number | null
    parent_id: number | null
    created_at: string
    // ... other fields
}

export interface Event {
    id: number
    task_id: number
    log_type: 'decision' | 'blocker' | 'milestone' | 'note'
    discussion_data: string
    timestamp: string
}

export type UnifiedSearchResult =
    | (Task & { result_type: 'task', match_snippet: string, match_field: string })
    | { result_type: 'event', event: Event, match_snippet: string, task_chain: Task[] }

export interface Project {
    name: string
    path: string
    is_online: boolean
    mcp_connected?: boolean
}

export interface PaginationState {
    page: number
    limit: number
    total: number
    totalPages: number
}

export const useAppStore = defineStore('app', () => {
    // State
    const isConnected = ref(false)
    const tasks = ref<Task[]>([])
    const events = ref<Event[]>([])
    const searchResults = ref<UnifiedSearchResult[]>([])
    const currentTaskId = ref<number | null>(null)
    const viewingTaskId = ref<number | null>(null)
    const projects = ref<Project[]>([])
    const currentProject = ref<Project | null>(null)
    const isCyberpunkMode = ref(false)
    const lastError = ref<string | null>(null)

    // Pagination
    const pagination = ref<PaginationState>({
        page: 1,
        limit: 200,
        total: 0,
        totalPages: 1
    })

    const searchPagination = ref<PaginationState>({
        page: 1,
        limit: 200,
        total: 0,
        totalPages: 1
    })

    // WebSocket
    let ws: WebSocket | null = null
    let reconnectTimer: any = null

    // Getters
    const taskTree = computed(() => {
        const map = new Map<number, any>()
        const roots: any[] = []

        // First pass: create nodes
        tasks.value.forEach(task => {
            map.set(task.id, { ...task, children: [] })
        })

        // Second pass: link children
        tasks.value.forEach(task => {
            const node = map.get(task.id)
            if (task.parent_id && map.has(task.parent_id)) {
                map.get(task.parent_id).children.push(node)
            } else {
                roots.push(node)
            }
        })

        // Post-sort: If we have a current task, move its root to the top
        if (currentTaskId.value) {
            // Find the root ancestor of the current task
            let current = map.get(currentTaskId.value)
            if (current) {
                while (current.parent_id && map.has(current.parent_id)) {
                    current = map.get(current.parent_id)
                }
                // 'current' is now the root ancestor
                const rootId = current.id

                // Sort roots: rootId comes first
                roots.sort((a, b) => {
                    if (a.id === rootId) return -1
                    if (b.id === rootId) return 1
                    return 0 // Keep original order for others
                })
            }
        }

        return roots
    })

    const currentTaskDetail = computed(() => {
        if (!viewingTaskId.value) return null
        return tasks.value.find(t => t.id === viewingTaskId.value) || null
    })

    // Actions
    function connect() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
        // In development (port 1393), connect directly to backend (port 3000) to avoid proxy issues
        const host = window.location.port === '1393'
            ? `${window.location.hostname}:3000`
            : window.location.host
        const url = `${protocol}//${host}/ws/ui`

        ws = new WebSocket(url)

        ws.onopen = () => {
            isConnected.value = true
            if (reconnectTimer) clearTimeout(reconnectTimer)

            // Send hello
            ws?.send(JSON.stringify({
                version: '1.0',
                type: 'hello',
                payload: { entity_type: 'web_ui', capabilities: null },
                timestamp: new Date().toISOString()
            }))
        }

        ws.onmessage = (event) => {
            try {
                const msg = JSON.parse(event.data)
                handleMessage(msg)
            } catch (e) {
                console.error('Failed to parse message:', e)
            }
        }

        ws.onclose = () => {
            isConnected.value = false
            ws = null
            reconnectTimer = setTimeout(() => {
                connect()
            }, 2000)
        }
    }

    function handleMessage(msg: any) {
        if (!msg) return
        switch (msg.type) {
            case 'init':
                projects.value = msg.payload.projects
                fetchCurrentTask().then(() => fetchTasks())
                break
            case 'task_created':
            case 'task_updated':
            case 'task_deleted':
                fetchTasks(undefined, undefined, pagination.value.page)
                if (viewingTaskId.value) fetchTaskDetail(viewingTaskId.value)
                break
            case 'event_created':
                if (viewingTaskId.value && msg.payload.data.task_id === viewingTaskId.value) {
                    fetchEvents(viewingTaskId.value)
                }
                break
            case 'db_operation':
                const op = msg.payload
                if (op.entity === 'task') {
                    fetchTasks(undefined, undefined, pagination.value.page)
                    // Always refresh current task state as it might have changed (e.g. start/done)
                    fetchCurrentTask()
                    if (viewingTaskId.value && op.affected_ids.includes(viewingTaskId.value)) {
                        if (op.operation !== 'delete') {
                            fetchTaskDetail(viewingTaskId.value)
                        }
                    }
                } else if (op.entity === 'event') {
                    if (viewingTaskId.value) {
                        fetchEvents(viewingTaskId.value)
                    }
                }
                break
            case 'project_online':
                {
                    const newProject = msg.payload.project
                    const idx = projects.value.findIndex(p => p.path === newProject.path)
                    if (idx >= 0) {
                        projects.value[idx] = newProject
                    } else {
                        projects.value.push(newProject)
                    }
                }
                break
            case 'project_offline':
                {
                    const path = msg.payload.project_path
                    const idx = projects.value.findIndex(p => p.path === path)
                    if (idx >= 0) {
                        if (currentProject.value?.path === path) {
                            projects.value[idx]!.is_online = false
                            projects.value[idx]!.mcp_connected = false
                        } else {
                            projects.value.splice(idx, 1)
                        }
                    }
                }
                break
        }
    }

    async function fetchTasks(status?: string, parentId?: number | null, page: number = 1) {
        try {
            const offset = (page - 1) * pagination.value.limit
            let url = `/api/tasks?offset=${offset}&limit=${pagination.value.limit}`
            if (status) url += `&status=${status}`
            if (parentId !== undefined) {
                url += `&parent=${parentId === null ? 'null' : parentId}`
            }

            const res = await fetch(url)
            if (!res.ok) throw new Error('Failed to fetch tasks')
            const json = await res.json()

            if (json.data.tasks) {
                tasks.value = json.data.tasks
                pagination.value = {
                    page: Math.floor(json.data.offset / json.data.limit) + 1,
                    limit: json.data.limit,
                    total: json.data.total_count,
                    totalPages: Math.ceil(json.data.total_count / json.data.limit)
                }
            } else {
                // Fallback for unexpected structure
                tasks.value = json.data
                pagination.value.total = json.data.length
            }

            // Validation and auto-select logic
            if (viewingTaskId.value && !tasks.value.find(t => t.id === viewingTaskId.value)) {
                viewingTaskId.value = null
            }
            if (tasks.value.length > 0 && !viewingTaskId.value) {
                if (tasks.value[0]) {
                    viewingTaskId.value = tasks.value[0].id
                    fetchTaskDetail(tasks.value[0].id)
                }
            }

            validateTaskOrder(tasks.value)
        } catch (e) {
            console.error('Failed to fetch tasks:', e)
            lastError.value = (e as Error).message
        }
    }

    function validateTaskOrder(taskList: Task[]) {
        if (taskList.length < 2) return

        for (let i = 0; i < taskList.length - 1; i++) {
            const current = taskList[i]
            const next = taskList[i + 1]
            if (!current || !next) continue

            // 1. Focus check (if current is focused, it should be first - but we might be past the first item)
            // Actually, if 'next' is focused and 'current' is not, that's an error.
            const isCurrentFocused = current.id === currentTaskId.value
            const isNextFocused = next.id === currentTaskId.value

            if (isNextFocused && !isCurrentFocused) {
                console.error(`[Sort Error] Focused task ${next.id} is not at the top. Found after ${current.id}`)
                continue
            }
            if (isCurrentFocused) continue // Focused task is allowed to be first

            // 2. Status check: doing < todo < done
            const statusOrder = { 'doing': 0, 'todo': 1, 'done': 2 }
            const currentStatusScore = statusOrder[current.status]
            const nextStatusScore = statusOrder[next.status]

            if (currentStatusScore > nextStatusScore) {
                console.error(`[Sort Error] Status order violated. ${current.status} (ID: ${current.id}) came before ${next.status} (ID: ${next.id})`)
                continue
            }
            if (currentStatusScore < nextStatusScore) continue

            // 3. Priority check: 1 < 2 < 3 < null (999)
            const getPriority = (p: number | null) => p === null ? 999 : p
            const currentPriority = getPriority(current.priority)
            const nextPriority = getPriority(next.priority)

            if (currentPriority > nextPriority) {
                console.error(`[Sort Error] Priority order violated. P${current.priority} (ID: ${current.id}) came before P${next.priority} (ID: ${next.id})`)
                continue
            }
            if (currentPriority < nextPriority) continue

            // 4. Creation Time (ID) check: Ascending
            if (current.id > next.id) {
                console.error(`[Sort Error] ID/Time order violated. ID ${current.id} came before ID ${next.id}`)
            }
        }
    }

    async function search(query: string, page: number = 1) {
        if (!query.trim()) {
            searchResults.value = []
            return
        }

        try {
            const offset = (page - 1) * searchPagination.value.limit
            const res = await fetch(`/api/search?query=${encodeURIComponent(query)}&offset=${offset}&limit=${searchPagination.value.limit}`)
            if (!res.ok) throw new Error('Search failed')
            const json = await res.json()

            if (json.data.results) {
                searchResults.value = json.data.results
                searchPagination.value = {
                    page: Math.floor(json.data.offset / json.data.limit) + 1,
                    limit: json.data.limit,
                    total: json.data.total_tasks + json.data.total_events, // Combined total? Or just tasks? User spec shows separate totals.
                    // For UI simplicity, let's use total_tasks + total_events for now, or just total_tasks if we only show tasks in tree.
                    // The UI currently only processes tasks from search results for the tree.
                    // Let's use total_tasks for now to be safe for the task tree.
                    // Actually, let's sum them up for the "Total" display.
                    totalPages: Math.ceil((json.data.total_tasks + json.data.total_events) / json.data.limit)
                }
                // Update total specifically
                searchPagination.value.total = json.data.total_tasks + json.data.total_events
            } else {
                searchResults.value = json.data
                searchPagination.value.total = json.data.length
            }
        } catch (e) {
            console.error('Search error:', e)
            lastError.value = (e as Error).message
            searchResults.value = []
        }
    }

    async function fetchCurrentTask() {
        try {
            const res = await fetch('/api/current-task', { cache: 'no-store' })
            if (!res.ok) throw new Error('Failed to fetch current task')
            const json = await res.json()
            if (json.data && json.data.task) {
                currentTaskId.value = json.data.task.id
                viewingTaskId.value = json.data.task.id
                fetchTaskDetail(json.data.task.id)
            } else {
                currentTaskId.value = null
            }
        } catch (e) {
            console.error('Failed to fetch current task:', e)
        }
    }

    async function fetchTaskDetail(id: number) {
        try {
            const res = await fetch(`/api/tasks/${id}`)
            const data = await res.json()
            const idx = tasks.value.findIndex(t => t.id === id)
            if (idx >= 0) {
                tasks.value[idx] = { ...tasks.value[idx], ...data.data }
            }
            fetchEvents(id)
        } catch (e) {
            console.error('Failed to fetch task detail:', e)
        }
    }

    async function fetchEvents(taskId: number) {
        try {
            const res = await fetch(`/api/tasks/${taskId}/events`)
            const data = await res.json()
            events.value = data.data || []
        } catch (e) {
            console.error('Failed to fetch events:', e)
        }
    }

    async function addTask(name: string, parentId?: number | null, priority?: number | null, _spec?: string) {
        try {
            await fetch('/api/tasks', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    name,
                    parent_id: parentId,
                    priority: priority,
                    spec: _spec
                })
            })
            fetchTasks(undefined, parentId, pagination.value.page)
        } catch (e) {
            console.error('Failed to add task:', e)
        }
    }

    async function updateTask(id: number, updates: Partial<Task>) {
        try {
            await fetch(`/api/tasks/${id}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(updates)
            })
            // Optimistic update
            const idx = tasks.value.findIndex(t => t.id === id)
            if (idx >= 0) {
                tasks.value[idx] = { ...tasks.value[idx], ...updates } as Task
            }
        } catch (e) {
            console.error('Failed to update task:', e)
        }
    }

    async function deleteTask(id: number) {
        try {
            await fetch(`/api/tasks/${id}`, {
                method: 'DELETE'
            })
            tasks.value = tasks.value.filter(t => t.id !== id)
            if (currentTaskId.value === id) currentTaskId.value = null
            if (viewingTaskId.value === id) viewingTaskId.value = null
            fetchTasks(undefined, undefined, pagination.value.page)
        } catch (e) {
            console.error('Failed to delete task:', e)
        }
    }

    async function startTask(id: number) {
        try {
            const res = await fetch(`/api/tasks/${id}/start`, { method: 'POST' })
            const json = await res.json()
            if (json.data) {
                const idx = tasks.value.findIndex(t => t.id === id)
                if (idx >= 0) {
                    tasks.value[idx] = { ...tasks.value[idx], ...json.data }
                }
                currentTaskId.value = id
                viewingTaskId.value = id
            }
        } catch (e) {
            console.error('Failed to start task:', e)
        }
    }

    async function doneTask() {
        try {
            const res = await fetch('/api/tasks/current/done', { method: 'POST' })
            const json = await res.json()
            if (json.data) {
                const idx = tasks.value.findIndex(t => t.id === json.data.id)
                if (idx >= 0) {
                    tasks.value[idx] = { ...tasks.value[idx], ...json.data }
                }
                currentTaskId.value = null
            }
        } catch (e) {
            console.error('Failed to complete task:', e)
        }
    }

    async function addEvent(taskId: number, type: string, data: string) {
        try {
            await fetch(`/api/tasks/${taskId}/events`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ event_type: type, data })
            })
            fetchEvents(taskId)
        } catch (e) {
            console.error('Failed to add event:', e)
        }
    }

    async function updateEvent(taskId: number, eventId: number, data: string) {
        try {
            await fetch(`/api/tasks/${taskId}/events/${eventId}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ data })
            })
            fetchEvents(taskId)
        } catch (e) {
            console.error('Failed to update event:', e)
        }
    }

    async function deleteEvent(taskId: number, eventId: number) {
        try {
            await fetch(`/api/tasks/${taskId}/events/${eventId}`, {
                method: 'DELETE'
            })
            fetchEvents(taskId)
        } catch (e) {
            console.error('Failed to delete event:', e)
        }
    }

    async function pickNextTask() {
        try {
            const response = await fetch('/api/tasks/next')
            if (!response.ok) throw new Error('Failed to pick next task')
            const json = await response.json()
            return json.data
        } catch (error) {
            console.error('Error picking next task:', error)
            lastError.value = (error as Error).message
        }
    }

    async function switchProject(path: string) {
        try {
            await fetch('/api/switch-project', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ project_path: path })
            })
            const targetProject = projects.value.find(p => p.path === path)
            if (targetProject) currentProject.value = targetProject

            tasks.value = []
            events.value = []
            currentTaskId.value = null
            viewingTaskId.value = null
            searchResults.value = []

            await fetchCurrentTask()
            await fetchTasks()
        } catch (e) {
            console.error('Failed to switch project:', e)
        }
    }

    function removeProject(path: string) {
        const idx = projects.value.findIndex(p => p.path === path)
        if (idx >= 0) projects.value.splice(idx, 1)
    }

    function toggleTheme() {
        isCyberpunkMode.value = !isCyberpunkMode.value
    }

    return {
        isConnected,
        tasks,
        events,
        searchResults,
        currentTaskId,
        viewingTaskId,
        projects,
        currentProject,
        isCyberpunkMode,
        lastError,
        pagination,
        searchPagination,
        taskTree,
        currentTaskDetail,
        connect,
        fetchTasks,
        fetchCurrentTask,
        fetchTaskDetail,
        fetchEvents,
        addTask,
        updateTask,
        deleteTask,
        startTask,
        doneTask,
        addEvent,
        updateEvent,
        deleteEvent,
        pickNextTask,
        search,
        switchProject,
        removeProject,
        toggleTheme
    }
})
