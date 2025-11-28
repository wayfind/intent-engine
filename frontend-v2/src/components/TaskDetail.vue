<script setup lang="ts">
import { computed, ref } from 'vue'
import { useAppStore } from '../stores/appStore'
import { useI18n } from '../composables/useI18n'
import InPlaceEditor from './InPlaceEditor.vue'
import ModalDialog from './ModalDialog.vue'
import TaskForm from './TaskForm.vue'
import MarkdownRenderer from './MarkdownRenderer.vue'
import { Trash2, GitBranch, Play, Square, ChevronDown, RotateCcw, Signal } from 'lucide-vue-next'

const store = useAppStore()
const { t } = useI18n()
const task = computed(() => store.currentTaskDetail)
const isStatusDropdownOpen = ref(false)
const isPriorityDropdownOpen = ref(false)
let priorityDropdownTimer: any = null
const isIdDropdownOpen = ref(false)
let idDropdownTimer: any = null
const showSubtaskModal = ref(false)

function updateName(newName: string) {
  if (!task.value) return
  store.updateTask(task.value.id, { name: newName })
}

function updateSpec(newSpec: string) {
  if (!task.value) return
  store.updateTask(task.value.id, { spec: newSpec })
}



function closeDropdown() {
  isStatusDropdownOpen.value = false
}

async function onStartTask() {
  console.error('onStartTask TRIGGERED')
  if (!task.value) return

  if (task.value.status === 'done') {
    // Reset to todo first
    await store.updateTask(task.value.id, { status: 'todo' })
    
    // Wait a bit for backend to process the state change before starting
    setTimeout(async () => {
      if (!task.value) return
      console.error('Starting task after delay')
      await store.startTask(task.value.id)
      store.currentTaskId = task.value.id
      store.currentTaskId = task.value.id // Set focus
    }, 200)
  } else {
    await store.startTask(task.value.id)
    store.currentTaskId = task.value.id // Set focus
  }
  
  closeDropdown()
}

async function onDoneTask() {
  console.log('onDoneTask called')
  await store.doneTask()
  closeDropdown()
}

function updateStatus(newStatus: 'todo' | 'doing' | 'done') {
  console.log('updateStatus called with:', newStatus)
  if (!task.value) return
  store.updateTask(task.value.id, { status: newStatus })
  closeDropdown()
}

function updatePriority(newPriority: number | null) {
  if (!task.value) return
  store.updateTask(task.value.id, { priority: newPriority })
  isPriorityDropdownOpen.value = false
}

function openPriorityDropdown() {
  if (priorityDropdownTimer) clearTimeout(priorityDropdownTimer)
  isPriorityDropdownOpen.value = true
}

function closePriorityDropdown() {
  priorityDropdownTimer = setTimeout(() => {
    isPriorityDropdownOpen.value = false
  }, 150)
}

function openIdDropdown() {
  if (idDropdownTimer) clearTimeout(idDropdownTimer)
  isIdDropdownOpen.value = true
}

function closeIdDropdown() {
  idDropdownTimer = setTimeout(() => {
    isIdDropdownOpen.value = false
  }, 150)
}

function deleteTask() {
  if (!task.value) return
  if (confirm(t('DELETE_TASK_CONFIRM', { name: task.value.name }))) {
    store.deleteTask(task.value.id)
  }
}

async function createSubtask(data: { name: string, parentId: number | null, priority: number | null, spec: string }) {
  if (!task.value) return
  await store.addTask(data.name, task.value.id, data.priority, data.spec)
  showSubtaskModal.value = false
}
</script>

<template>
  <div v-if="task" class="flex-1 h-full overflow-y-auto bg-sci-base relative scrollbar-thin scrollbar-thumb-sci-border scrollbar-track-transparent animate-fade-in">
    <!-- Background Grid (Only in Dark Mode via CSS) -->
    <div class="absolute inset-0 pointer-events-none opacity-[0.03] bg-grid-pattern"></div>

    <div class="w-full h-full p-4 relative z-10 flex flex-col">
      <!-- Header -->
      <div class="mb-6 group/header">
        <!-- Metadata Row (ID, Status, Priority, Actions) -->
        <!-- Metadata Row (ID, Status, Priority, Actions) -->
        <!-- Metadata Row (ID, Status, Priority, Actions) -->
        <div class="flex items-center gap-8 mb-4 text-xs font-mono text-sci-text-dim">
          <!-- ID Dropdown -->
          <div class="relative" @mouseenter="openIdDropdown" @mouseleave="closeIdDropdown">
            <button 
              class="flex items-center gap-2 bg-sci-base border border-sci-border rounded-sm px-3 h-7 hover:bg-sci-panel-hover hover:border-sci-text-dim hover:text-sci-text-pri transition-all font-mono"
            >
              <span class="opacity-50">#</span>
              <span>{{ task.id.toString().padStart(4, '0') }}</span>
              <ChevronDown class="w-3 h-3 opacity-30" />
            </button>

            <div v-if="isIdDropdownOpen" class="absolute left-0 mt-1 w-40 bg-sci-base border border-sci-border shadow-xl rounded-sm z-50 py-1 backdrop-blur-sm flex flex-col">
              <button 
                @click="showSubtaskModal = true; closeIdDropdown()"
                class="w-full flex items-center gap-2 px-3 py-2 text-xs font-mono text-left hover:bg-sci-panel-hover hover:text-sci-cyan transition-colors"
              >
                <GitBranch class="w-3.5 h-3.5" />
                {{ t('SPAWN_SUBTASK') }}
              </button>
              <div class="h-px bg-sci-border my-1 opacity-50"></div>
              <button 
                @click="deleteTask(); closeIdDropdown()"
                class="w-full flex items-center gap-2 px-3 py-2 text-xs font-mono text-left hover:bg-sci-panel-hover hover:text-sci-danger transition-colors text-sci-danger"
              >
                <Trash2 class="w-3.5 h-3.5" />
                {{ t('DELETE') }}
              </button>
            </div>
          </div>
          
          <!-- Status -->
          <div class="relative" @mouseenter="isStatusDropdownOpen = true" @mouseleave="isStatusDropdownOpen = false">
            <button 
              class="flex items-center gap-2 bg-sci-base border border-sci-border rounded-sm px-3 h-7 hover:bg-sci-panel-hover hover:border-sci-text-dim hover:text-sci-text-pri transition-all uppercase tracking-wider font-bold"
              :class="{
                'text-sci-text-dim': task.status === 'todo' || task.status === 'done',
                'text-sci-cyan': task.status === 'doing'
              }"
            >
              <div v-if="task.status === 'doing'" class="animate-pulse">
                <Play class="w-[18px] h-[18px] fill-current" />
              </div>
              
              <div v-else-if="task.status === 'done'" class="relative w-[18px] h-[18px]">
                <Square class="w-full h-full" />
                <span class="absolute -top-[2px] right-0 flex items-center justify-center font-serif font-bold text-lg leading-none select-none">✓</span>
              </div>
              
              <Square v-else class="w-[18px] h-[18px]" />
              
              <span>{{ task.status }}</span>
              <ChevronDown class="w-3 h-3 opacity-30" />
            </button>

            <!-- Dropdown Menu (Keep existing logic, just styling tweaks if needed) -->
            <div v-if="isStatusDropdownOpen" class="absolute top-full left-0 w-48 z-30 pt-2">
              <div class="bg-sci-base border border-sci-border rounded-sm p-1 backdrop-blur-sm">
                 <!-- ... keep existing menu items ... -->
                  <template v-if="task.status === 'todo'">
                    <button @click="onStartTask" class="w-full flex items-center gap-2 px-2 py-2 text-xs font-mono text-left hover:bg-sci-panel-hover hover:text-sci-cyan rounded-sm transition-colors group">
                      <Play class="w-3 h-3 group-hover:fill-current" />
                      {{ t('START') }}
                    </button>
                    <button @click="updateStatus('done')" class="w-full flex items-center gap-2 px-2 py-2 text-xs font-mono text-left hover:bg-sci-panel-hover hover:text-sci-text-pri rounded-sm transition-colors">
                      <div class="relative w-[18px] h-[18px] text-sci-text-dim">
                        <Square class="w-full h-full" />
                        <span class="absolute -top-[2px] right-0 flex items-center justify-center font-serif font-bold text-lg leading-none select-none">✓</span>
                      </div>
                      {{ t('MARK_DONE') }}
                    </button>
                  </template>

                  <template v-if="task.status === 'doing'">
                    <button 
                      @click="task.id === store.currentTaskId ? onDoneTask() : updateStatus('done')" 
                      class="w-full flex items-center gap-2 px-2 py-2 text-xs font-mono text-left hover:bg-sci-panel-hover hover:text-sci-text-pri rounded-sm transition-colors"
                    >
                      <div class="relative w-[18px] h-[18px] text-sci-text-dim">
                        <Square class="w-full h-full" />
                        <span class="absolute -top-[2px] right-0 flex items-center justify-center font-serif font-bold text-lg leading-none select-none">✓</span>
                      </div>
                      {{ t('DONE') }}
                    </button>
                    <button @click="updateStatus('todo')" class="w-full flex items-center gap-2 px-2 py-2 text-xs font-mono text-left hover:bg-sci-panel-hover hover:text-sci-text-pri rounded-sm transition-colors">
                      <Square class="w-[18px] h-[18px] text-sci-text-dim" />
                      {{ t('STOP') }}
                    </button>
                  </template>

                  <template v-if="task.status === 'done'">
                    <button @click="onStartTask" class="w-full flex items-center gap-2 px-2 py-2 text-xs font-mono text-left hover:bg-sci-panel-hover hover:text-sci-cyan rounded-sm transition-colors group">
                      <Play class="w-3 h-3 group-hover:fill-current" />
                      {{ t('RESTART') }}
                    </button>
                    <button @click="updateStatus('todo')" class="w-full flex items-center gap-2 px-2 py-2 text-xs font-mono text-left hover:bg-sci-panel-hover hover:text-sci-text-main rounded-sm transition-colors">
                      <RotateCcw class="w-3 h-3" />
                      {{ t('MARK_TODO') }}
                    </button>
                  </template>
              </div>
            </div>
          </div>

          <!-- Priority -->
          <div class="flex items-center gap-2">
            <div class="relative" @mouseenter="openPriorityDropdown" @mouseleave="closePriorityDropdown">
              <button 
                class="flex items-center gap-2 text-xs font-mono bg-sci-base border border-sci-border rounded-sm px-3 h-7 hover:bg-sci-panel-hover hover:border-sci-text-dim hover:text-sci-text-pri transition-all uppercase tracking-wider font-bold"
              >
                <Signal class="w-[18px] h-[18px]" :class="{
                  'text-sci-danger': task.priority === 1,
                  'text-sci-orange': task.priority === 2,
                  'text-sci-cyan': task.priority === 3,
                  'text-sci-text-dim': !task.priority || task.priority > 3
                }" />
                <span :class="{
                  'text-sci-danger': task.priority === 1,
                  'text-sci-orange': task.priority === 2,
                  'text-sci-cyan': task.priority === 3,
                  'text-sci-text-dim': !task.priority || task.priority > 3
                }">
                  {{ 
                    task.priority === 1 ? t('CRITICAL') :
                    task.priority === 2 ? t('HIGH') :
                    task.priority === 3 ? t('MEDIUM') :
                    task.priority === 4 ? t('LOW') :
                    t('NONE')
                  }}
                </span>
                <ChevronDown class="w-3 h-3 opacity-30" />
              </button>

              <div v-if="isPriorityDropdownOpen" class="absolute left-0 mt-1 w-32 bg-sci-base border border-sci-border shadow-xl rounded-sm z-50 py-1 backdrop-blur-sm flex flex-col">
                <button 
                  @click="updatePriority(1)" 
                  class="w-full text-left px-3 py-1.5 text-xs font-mono text-sci-danger hover:bg-sci-panel-hover transition-colors"
                >
                  {{ t('CRITICAL') }}
                </button>
                <button 
                  @click="updatePriority(2)" 
                  class="w-full text-left px-3 py-1.5 text-xs font-mono text-sci-orange hover:bg-sci-panel-hover transition-colors"
                >
                  {{ t('HIGH') }}
                </button>
                <button 
                  @click="updatePriority(3)" 
                  class="w-full text-left px-3 py-1.5 text-xs font-mono text-sci-cyan hover:bg-sci-panel-hover transition-colors"
                >
                  {{ t('MEDIUM') }}
                </button>
                <button 
                  @click="updatePriority(4)" 
                  class="w-full text-left px-3 py-1.5 text-xs font-mono text-sci-text-dim hover:bg-sci-panel-hover transition-colors"
                >
                  {{ t('LOW') }}
                </button>
                <button 
                  @click="updatePriority(null)" 
                  class="w-full text-left px-3 py-1.5 text-xs font-mono text-sci-text-dim hover:bg-sci-panel-hover transition-colors"
                >
                  {{ t('NONE') }}
                </button>
              </div>
            </div>
          </div>

          </div>

        
        <InPlaceEditor 
          :model-value="task.name || ''" 
          @save="updateName"
          class="text-4xl font-display font-bold tracking-tight leading-tight transition-colors"
          :class="{
            'text-sci-danger': task.priority === 1,
            'text-sci-orange': task.priority === 2,
            'text-sci-cyan': task.priority === 3,
            'text-sci-text-pri': !task.priority || task.priority > 3
          }"
        />
      </div>

      <!-- Spec (Borderless, clean) -->
      <div class="mb-12 flex-1 flex flex-col">
        <InPlaceEditor 
          :model-value="task.spec || null" 
          multiline 
          markdown
          :placeholder="t('NO_SPEC')"
          @save="updateSpec"
          class="flex-1"
        >
          <template #display>
            <MarkdownRenderer 
              :source="task.spec || null" 
              :placeholder="t('NO_SPEC')"
            />
          </template>
        </InPlaceEditor>
      </div>
    </div>

    <!-- Subtask Modal -->
    <ModalDialog 
      :is-open="showSubtaskModal" 
      :title="t('SPAWN_SUBTASK')"
      size="4xl"
      @close="showSubtaskModal = false"
    >
      <TaskForm 
        v-if="task"
        :parent-id="task.id"
        :parent-name="task.name"
        @submit="createSubtask"
        @cancel="showSubtaskModal = false"
      />
    </ModalDialog>
  </div>
  
  <div v-else class="flex-1 flex items-center justify-center bg-sci-base relative overflow-hidden">
    <!-- Empty State -->
    <div class="absolute inset-0 pointer-events-none opacity-[0.02]"
         style="background-image: linear-gradient(var(--text-main) 1px, transparent 1px), linear-gradient(90deg, var(--text-main) 1px, transparent 1px); background-size: 40px 40px;">
    </div>
    
    <div class="text-center text-sci-text-dim relative z-10">
      <div class="w-20 h-20 border border-sci-border border-t-sci-cyan rounded-full animate-spin mx-auto mb-6"></div>
      <p class="font-mono text-sm tracking-widest uppercase">{{ t('AWAITING_SELECTION') }}</p>
      <p class="text-xs mt-2 opacity-50">SELECT A NODE TO INITIALIZE</p>
    </div>
  </div>
</template>
