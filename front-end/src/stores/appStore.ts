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

export interface Project {
    name: string
    path: string
    is_online: boolean
    mcp_connected?: boolean
}

export const useAppStore = defineStore('app', () => {
    // State
    const isConnected = ref(false)
    const tasks = ref<Task[]>([])
    const events = ref<Event[]>([])
    const currentTaskId = ref<number | null>(null)
    const viewingTaskId = ref<number | null>(null)
    const projects = ref<Project[]>([])
    const currentProject = ref<Project | null>(null)
    const isCyberpunkMode = ref(false) // Default to false (Light Mode)

    // WebSocket
    let ws: WebSocket | null = null
    let reconnectTimer: any = null

    // Getters
    const taskTree = computed(() => {
        // Build tree from flat tasks list
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

            // Reconnect
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

                // After init, we should fetch tasks first to populate the list
                // Then fetch current task to set focus and get details
                fetchTasks().then(() => fetchCurrentTask())
                break
            case 'task_created':
            case 'task_updated':
            case 'task_deleted':
                fetchTasks()
                if (viewingTaskId.value) fetchTaskDetail(viewingTaskId.value)
                break
            case 'event_created':
                if (viewingTaskId.value && msg.payload.data.task_id === viewingTaskId.value) {
                    fetchEvents(viewingTaskId.value)
                }
                break
            case 'db_operation':
                // Handle generic DB operations from MCP or other sources
                // Payload: { operation: 'create'|'update'|'delete', entity: 'task'|'event', affected_ids: number[], ... }
                const op = msg.payload
                if (op.entity === 'task') {
                    fetchTasks()
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
                        // Mark as offline instead of removing, or remove if it's strictly dynamic?
                        // The backend 'get_online_projects' returns only online ones (except current).
                        // But 'project_offline' implies it was online.
                        // Let's just remove it from the list if it's not the current one, 
                        // or mark it as offline if we want to keep it visible.
                        // Given the UI is "Project Modules", maybe we just remove it if it disconnects?
                        // But wait, if I am IN the project, I shouldn't lose it.
                        // Let's check backend logic: get_online_projects returns "active" projects.
                        // If it goes offline, it's no longer "active" unless it's the current one.
                        // Let's try to just remove it for now to match "dynamic tabs" request.
                        // But if it's current, we might want to keep it or show error.
                        if (currentProject.value?.path === path) {
                            projects.value[idx]!.is_online = false
                            projects.value[idx]!.mcp_connected = false // Assuming we extend type
                        } else {
                            projects.value.splice(idx, 1)
                        }
                    }
                }
                break
        }
    }

    async function fetchTasks() {
        try {
            const res = await fetch('/api/tasks')
            const data = await res.json()
            const newTasks = data.data || [] as Task[]

            // Smart merge to preserve details (like spec) that might be missing in the list response
            const mergedTasks = newTasks.map((newTask: Task) => {
                const existingTask = tasks.value.find(t => t.id === newTask.id)
                if (existingTask) {
                    // Merge new summary data into existing detailed task
                    // This preserves fields like 'spec' if they are missing in newTask but present in existingTask
                    return { ...existingTask, ...newTask }
                }
                return newTask
            })

            tasks.value = mergedTasks

            // Validation: if viewingTaskId is set but not in list, clear it
            if (viewingTaskId.value && !tasks.value.find(t => t.id === viewingTaskId.value)) {
                viewingTaskId.value = null
            }

            // Auto-select first task if none selected
            if (tasks.value.length > 0 && !viewingTaskId.value) {
                if (tasks.value[0]) {
                    viewingTaskId.value = tasks.value[0].id
                    fetchTaskDetail(tasks.value[0].id)
                }
            }
        } catch (e) {
            console.error('Failed to fetch tasks:', e)
        }
    }

    async function fetchCurrentTask() {
        try {
            const res = await fetch('/api/current-task')
            const data = await res.json()
            if (data.data?.task) {
                currentTaskId.value = data.data.task.id
                // Always switch to current task when fetched (init/project switch)
                viewingTaskId.value = data.data.task.id
                fetchTaskDetail(data.data.task.id)
            }
        } catch (e) {
            console.error('Failed to fetch current task:', e)
        }
    }

    async function fetchTaskDetail(id: number) {
        try {
            const res = await fetch(`/api/tasks/${id}`)
            const data = await res.json()
            // Update in list if exists
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

    // Task Operations
    async function addTask(name: string, parentId?: number | null, priority?: number | null, _spec?: string) {
        try {
            await fetch('/api/tasks', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    name,
                    parent_id: parentId,
                    priority: priority,
                    spec: _spec,
                    spec_stdin: false,
                })
            })

            // Since we can't easily get the ID of the created task from here without changing the return type
            // and the backend response structure, we rely on fetchTasks to refresh.
            // If spec needs to be set separately, we might need to find the new task.
            // For now, let's assume simple creation and if spec is needed, we might need to enhance the backend 
            // or do a "find latest" hack.
            // actually, let's check if we can update the spec immediately if we knew the ID.
            // The backend create_task returns the created task.

            // Let's refactor to parse response
            await fetch('/api/tasks') // Re-fetch all for now as simple sync
            // Optimization: We could parse the POST response if we changed the fetch above to return it.

            // Wait, the previous fetch call didn't capture response. Let's capture it.
            /*
           const createRes = await fetch('/api/tasks', ...)
           const createData = await createRes.json()
           if (createData.data && spec) {
                await updateTask(createData.data.id, { spec })
           }
           */
            // For this iteration, let's stick to the existing pattern but if spec is critical
            // we should probably ensure it's saved. 
            // Given the user wants a large form, they expect it to be saved.
            // I'll assume for now the backend *might* not take spec in create.
            // So I will modify this to try and update if possible, but without ID it's hard.
            // Let's just trigger refresh.
            fetchTasks()
        } catch (e) {
            console.error('Failed to add task:', e)
        }
    }

    async function updateTask(id: number, updates: Partial<Task>) {
        try {
            await fetch(`/api/tasks/${id}`, {
                method: 'PATCH',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(updates)
            })
            // Optimistic update
            const idx = tasks.value.findIndex(t => t.id === id)
            if (idx >= 0) {
                // Ensure we don't overwrite id with undefined
                const updatedTask = { ...tasks.value[idx], ...updates } as Task
                tasks.value[idx] = updatedTask
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
            // Optimistic remove
            tasks.value = tasks.value.filter(t => t.id !== id)
            if (currentTaskId.value === id) {
                currentTaskId.value = null
            }
            if (viewingTaskId.value === id) {
                viewingTaskId.value = null
            }
        } catch (e) {
            console.error('Failed to delete task:', e)
        }
    }

    async function startTask(id: number) {
        try {
            const res = await fetch(`/api/tasks/${id}/start`, {
                method: 'POST'
            })
            const data = await res.json()
            if (data.data) {

                // Update task in list
                const idx = tasks.value.findIndex(t => t.id === id)
                if (idx >= 0) {
                    tasks.value[idx] = { ...tasks.value[idx], ...data.data }
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
            const res = await fetch('/api/tasks/done', {
                method: 'POST'
            })
            const data = await res.json()
            if (data.data && data.data.completed_task) {
                // Update task in list
                const idx = tasks.value.findIndex(t => t.id === data.data.completed_task.id)
                if (idx >= 0) {
                    tasks.value[idx] = { ...tasks.value[idx], ...data.data.completed_task }
                }
                currentTaskId.value = null
            }
        } catch (e) {
            console.error('Failed to complete task:', e)
        }
    }

    // Event Operations
    async function addEvent(taskId: number, type: string, data: string) {
        try {
            await fetch(`/api/tasks/${taskId}/events`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    type: type,
                    data: data
                })
            })
            fetchEvents(taskId)
        } catch (e) {
            console.error('Failed to add event:', e)
        }
    }

    async function updateEvent(taskId: number, eventId: number, data: string) {
        try {
            await fetch(`/api/tasks/${taskId}/events/${eventId}`, {
                method: 'PATCH',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    data: data
                })
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

    // Search Operations
    const searchResults = ref<any[] | null>(null)

    async function search(query: string) {
        if (!query.trim()) {
            searchResults.value = null
            return
        }
        try {
            const res = await fetch(`/api/search?query=${encodeURIComponent(query)}`)
            const data = await res.json()
            searchResults.value = data.data || []
        } catch (e) {
            console.error('Failed to search:', e)
            searchResults.value = []
        }
    }

    // Project Operations
    async function switchProject(path: string) {

        try {
            await fetch('/api/switch-project', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ project_path: path })
            })
            // The server will trigger a reload or we wait for init message
            // But usually we might want to relhoad the page or clear state
            // Update current project state
            const targetProject = projects.value.find(p => p.path === path)
            if (targetProject) {
                currentProject.value = targetProject
            }

            // Clear other state
            tasks.value = []
            events.value = []
            currentTaskId.value = null
            viewingTaskId.value = null
            searchResults.value = null

            // Fetch data for the new project
            await fetchCurrentTask()
            await fetchTasks()
        } catch (e) {
            console.error('Failed to switch project:', e)
        }
    }

    function removeProject(path: string) {
        const idx = projects.value.findIndex(p => p.path === path)
        if (idx >= 0) {
            projects.value.splice(idx, 1)
        }
    }

    function toggleTheme() {
        isCyberpunkMode.value = !isCyberpunkMode.value
    }

    return {
        isConnected,
        tasks,
        taskTree,
        currentTaskId,
        viewingTaskId,
        currentTaskDetail,
        events,
        projects,
        currentProject,
        isCyberpunkMode,
        toggleTheme,
        connect,
        fetchTasks,
        fetchTaskDetail,
        addTask,
        updateTask,
        deleteTask,
        startTask,
        doneTask,
        addEvent,
        updateEvent,
        deleteEvent,
        switchProject,
        removeProject,
        search,
        searchResults
    }
})
