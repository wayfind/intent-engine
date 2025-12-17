<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useAppStore, type Task } from '../stores/appStore'
import { useI18n } from '../composables/useI18n'
import TaskTreeItem from './TaskTreeItem.vue'
import ModalDialog from './ModalDialog.vue'
import TaskForm from './TaskForm.vue'
import { Search, Plus, ChevronLeft, ChevronRight } from 'lucide-vue-next'
import { useDebounceFn } from '@vueuse/core'

const store = useAppStore()
const { t } = useI18n()

const searchQuery = ref('')
const showAddTaskModal = ref(false)

// Debounced search
const debouncedSearch = useDebounceFn((query: string) => {
  store.search(query, 1) // Reset to page 1 on new search
}, 500)

watch(searchQuery, (newVal) => {
  debouncedSearch(newVal)
})

// Pagination Logic
const currentPagination = computed(() => {
  return searchQuery.value.trim() ? store.searchPagination : store.pagination
})

function handlePageChange(newPage: number) {
  if (newPage < 1 || newPage > currentPagination.value.totalPages) return
  
  if (searchQuery.value.trim()) {
    store.search(searchQuery.value, newPage)
  } else {
    store.fetchTasks(undefined, undefined, newPage)
  }
}

// Tree Logic
const displayedTree = computed(() => {
  // If searching, build tree from search results
  if (searchQuery.value.trim()) {
    if (store.searchResults.length > 0) {
      const tasks: Task[] = []
      
      // Extract tasks from search results
      store.searchResults.forEach(result => {
        if (result.result_type === 'task') {
          tasks.push(result)
        }
        // If it's an event, we might want to show the task it belongs to?
        // For now, let's focus on tasks as per requirement.
      })

      // Build tree from these tasks
      const map = new Map<number, any>()
      const roots: any[] = []

      // 1. Create nodes
      tasks.forEach(task => {
        map.set(task.id, { ...task, children: [], isOpen: true }) // Default open for search results
      })

      // 2. Link children
      tasks.forEach(task => {
        const node = map.get(task.id)
        if (task.parent_id && map.has(task.parent_id)) {
          map.get(task.parent_id).children.push(node)
        } else {
          roots.push(node)
        }
      })

      return roots
    } else {
      return []
    }
  }
  
  // Default: use store's taskTree (which is already built from current page of tasks)
  return store.taskTree
})

async function handleCreateTask(data: { name: string, parentId: number | null, priority: number | null, spec: string }) {
  await store.addTask(data.name, data.parentId, data.priority, data.spec)
  showAddTaskModal.value = false
}
</script>

<template>
  <aside class="flex flex-col h-full bg-sci-panel border-r border-sci-border relative z-10">
    <!-- Search & Actions -->
    <div class="p-4 border-b border-sci-border space-y-3">
      <!-- Search Input -->
      <div class="relative group">
        <div class="absolute inset-0 bg-sci-cyan opacity-0 group-focus-within:opacity-10 transition-opacity rounded blur-sm"></div>
        <Search class="absolute left-3 top-2.5 w-4 h-4 text-sci-text-dim group-focus-within:text-sci-cyan transition-colors" />
        <input 
          v-model="searchQuery"
          type="text" 
          :placeholder="t('SEARCH_TASKS')" 
          class="relative z-10 w-full bg-sci-base border border-sci-border rounded pl-9 pr-3 py-2 text-sm font-mono text-sci-text-pri focus:outline-none focus:border-sci-cyan transition-all placeholder-sci-text-dim"
        >
        <div class="absolute right-2 top-2.5 text-[10px] font-mono text-sci-text-dim border border-sci-border px-1 rounded opacity-50">/</div>
      </div>

      <!-- Add Task Button -->
      <button 
        @click="showAddTaskModal = true"
        class="flex items-center justify-center w-full py-2 bg-sci-cyan-dim border border-sci-cyan text-sci-cyan rounded hover:bg-sci-cyan hover:text-sci-base transition-all text-sm font-mono gap-2 group relative overflow-hidden"
        :title="t('NEW_TASK')"
      >
        <div class="absolute inset-0 bg-sci-cyan opacity-0 group-hover:opacity-10 transition-opacity"></div>
        <Plus class="w-4 h-4" />
        <span class="tracking-wider font-bold">{{ t('NEW_TASK') }}</span>
      </button>
    </div>

    <!-- Tree -->
    <div class="flex-1 overflow-y-auto py-2 scrollbar-thin scrollbar-thumb-sci-border scrollbar-track-transparent animate-fade-in">
      <div v-if="displayedTree.length === 0" class="p-8 text-center text-sci-text-dim text-xs font-mono">
        {{ searchQuery.trim() ? 'NO_RESULTS' : 'NO_DATA_STREAM' }}
      </div>
      <TaskTreeItem 
        v-for="root in displayedTree" 
        :key="root.id" 
        :node="root" 
        :force-open="!!searchQuery.trim()"
      />
    </div>
    
    <!-- Footer / Pagination -->
    <div class="p-3 border-t border-sci-border bg-sci-base/50 backdrop-blur text-[10px] font-mono text-sci-text-dim flex flex-col gap-2">
      <!-- Stats -->
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="flex items-center gap-1.5" :title="store.isConnected ? 'Connected' : 'Disconnected'">
            <div class="w-1.5 h-1.5 rounded-full" :class="store.isConnected ? 'bg-sci-success' : 'bg-sci-danger'"></div>
          </div>
          <span>{{ t('TOTAL_TASKS', { count: currentPagination.total.toString() }) }}</span>
        </div>
      </div>

      <!-- Pagination Controls -->
      <div class="flex items-center justify-between pt-2 border-t border-sci-border/50">
        <button 
          @click="handlePageChange(currentPagination.page - 1)"
          :disabled="currentPagination.page <= 1"
          class="p-1 hover:bg-sci-cyan/10 hover:text-sci-cyan rounded disabled:opacity-30 disabled:hover:bg-transparent disabled:hover:text-inherit transition-colors"
        >
          <ChevronLeft class="w-3 h-3" />
        </button>
        
        <span class="text-sci-cyan">
          PAGE {{ currentPagination.page }} / {{ Math.max(1, currentPagination.totalPages) }}
        </span>

        <button 
          @click="handlePageChange(currentPagination.page + 1)"
          :disabled="currentPagination.page >= currentPagination.totalPages"
          class="p-1 hover:bg-sci-cyan/10 hover:text-sci-cyan rounded disabled:opacity-30 disabled:hover:bg-transparent disabled:hover:text-inherit transition-colors"
        >
          <ChevronRight class="w-3 h-3" />
        </button>
      </div>
    </div>

    <!-- Add Task Modal -->
    <ModalDialog 
      :is-open="showAddTaskModal" 
      :title="t('NEW_TASK')"
      size="4xl"
      @close="showAddTaskModal = false"
    >
      <TaskForm 
        @submit="handleCreateTask"
        @cancel="showAddTaskModal = false"
      />
    </ModalDialog>
  </aside>
</template>
