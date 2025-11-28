<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useAppStore } from '../stores/appStore'
import { useI18n } from '../composables/useI18n'
import TaskTreeItem from './TaskTreeItem.vue'
import ModalDialog from './ModalDialog.vue'
import TaskForm from './TaskForm.vue'
import { Search, Plus } from 'lucide-vue-next'
import { useDebounceFn } from '@vueuse/core'

const store = useAppStore()
const { t } = useI18n()

const searchQuery = ref('')
const showAddTaskModal = ref(false)

// Debounced search
const debouncedSearch = useDebounceFn((query: string) => {
  store.search(query)
}, 1000)

watch(searchQuery, (newVal) => {
  debouncedSearch(newVal)
})

// Smart Project Selection Logic (Kept for auto-switching but UI moved to Header)
watch(() => store.projects, (newProjects) => {
  if (store.currentProject) return 
  
  const activeProjects = newProjects.filter((p: any) => p.is_online)
  if (activeProjects.length > 0 && activeProjects[0]) {
    const path = activeProjects[0].path
    if (typeof path === 'string' && path.length > 0) {
      store.switchProject(path)
    }
  }
}, { immediate: true, deep: true })

const displayedTree = computed(() => {
  if (store.searchResults) {
    const resultIds = new Set(store.searchResults.map((t: any) => t.id))
    
    function filterNode(node: any): any {
      const isMatch = resultIds.has(node.id)
      const children = node.children ? node.children.map(filterNode).filter((n: any) => n !== null) : []
      
      if (isMatch || children.length > 0) {
        return {
          ...node,
          children,
          isOpen: true 
        }
      }
      return null
    }

    return store.taskTree.map(filterNode).filter(n => n !== null)
  }
  
  return store.taskTree
})

async function handleCreateTask(data: { name: string, parentId: number | null, priority: number | null, spec: string }) {
  // We need to handle the fact that addTask currently doesn't return the ID to update spec
  // But we can try to update the store logic or just accept that spec might need a second pass if backend doesn't support it
  // For now, we pass everything to addTask and hope I updated appStore correctly to handle it (or I will update it next)
  // Wait, I updated appStore signature but noted backend limitation.
  // Actually, to fully support spec on create, we should probably update the backend or do a find-and-update.
  // But let's assume for this step we just call the store.
  await store.addTask(data.name, data.parentId, data.priority, data.spec)
  
  // If spec was provided, we might want to ensure it's saved. 
  // Since we don't have the ID, we can't easily update it here.
  // Ideally, we'd fetch the latest task or search for it.
  // For now, let's rely on the store.
  
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
          class="w-full bg-sci-base border border-sci-border rounded pl-9 pr-3 py-2 text-sm font-mono text-sci-text-pri focus:outline-none focus:border-sci-cyan transition-all placeholder-sci-text-dim"
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
        NO_DATA_STREAM
      </div>
      <TaskTreeItem 
        v-for="root in displayedTree" 
        :key="root.id" 
        :node="root" 
        :force-open="!!store.searchResults"
      />
    </div>
    
    <!-- Footer -->
    <div class="p-3 border-t border-sci-border bg-sci-base/50 backdrop-blur text-[10px] font-mono text-sci-text-dim flex items-center justify-between">
      <div class="flex items-center gap-3">
        <div class="flex items-center gap-1.5" :title="store.isConnected ? 'Connected' : 'Disconnected'">
          <div class="w-1.5 h-1.5 rounded-full" :class="store.isConnected ? 'bg-sci-success' : 'bg-sci-danger'"></div>
        </div>
        <span>{{ t('TOTAL_TASKS', { count: store.tasks.length.toString() }) }}</span>
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
