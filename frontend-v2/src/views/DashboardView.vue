<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { useAppStore } from '../stores/appStore'
import Sidebar from '../components/Sidebar.vue'
import TaskDetail from '../components/TaskDetail.vue'
import EventLog from '../components/EventLog.vue'
import SystemHeader from '../components/SystemHeader.vue'

const store = useAppStore()

// Split pane logic
const containerRef = ref<HTMLElement | null>(null)
const detailRatio = ref(0.45) // Task:Event = 4.5:5.5
const isDraggingRight = ref(false)

function startDragRight() {
  isDraggingRight.value = true
  document.addEventListener('mousemove', onDragRight)
  document.addEventListener('mouseup', stopDrag)
  document.body.style.cursor = 'col-resize'
  document.body.style.userSelect = 'none'
}

function onDragRight(e: MouseEvent) {
  if (!containerRef.value) return
  const rect = containerRef.value.getBoundingClientRect()
  const x = e.clientX - rect.left
  
  // Sidebar is fixed 280px
  const sidebarWidth = 280
  const remainingWidth = rect.width - sidebarWidth
  
  // Calculate position within remaining space
  const middleWidth = x - sidebarWidth
  
  // Calculate percentage of remaining space
  const newDetailRatio = Math.max(0.2, Math.min(0.8, middleWidth / remainingWidth))
  detailRatio.value = newDetailRatio
}

function stopDrag() {
  isDraggingRight.value = false
  document.removeEventListener('mousemove', onDragRight)
  document.removeEventListener('mouseup', stopDrag)
  document.body.style.cursor = ''
  document.body.style.userSelect = ''
}

const detailStyle = computed(() => ({ flex: `0 0 ${detailRatio.value * 100}%` }))
// Right pane takes remaining space automatically via flex-1

onMounted(() => {
  store.connect()
})
</script>

<template>
  <div class="flex flex-col h-screen w-screen overflow-hidden bg-sci-base text-sci-text-pri font-sans">
    <SystemHeader />
    
    <main ref="containerRef" class="flex-1 flex overflow-hidden relative">
      <!-- Left Pane (Sidebar) - Fixed Width -->
      <div class="relative w-[280px] flex-none flex flex-col border-r border-sci-border">
        <Sidebar />
      </div>

      <!-- Content Area (Task + Event) -->
      <div class="flex-1 flex overflow-hidden relative">
        <!-- Middle Pane (Task Detail) -->
        <div :style="detailStyle" class="relative min-w-[300px] flex flex-col bg-sci-base z-0">
          <!-- Grid Background -->
          <div class="absolute inset-0 pointer-events-none opacity-[0.03]"
               style="background-image: linear-gradient(var(--text-main) 1px, transparent 1px), linear-gradient(90deg, var(--text-main) 1px, transparent 1px); background-size: 40px 40px;">
          </div>
          <TaskDetail />
        </div>

        <!-- Resizer Right -->
        <div 
          @mousedown="startDragRight"
          class="w-1 bg-sci-border hover:bg-sci-cyan cursor-col-resize transition-colors relative z-20 flex items-center justify-center group"
        >
          <div class="absolute inset-y-0 -left-1 -right-1 z-10"></div> <!-- Hit area -->
        </div>

        <!-- Right Pane (Event Log) -->
        <div class="flex-1 relative min-w-[200px] flex flex-col bg-sci-panel border-l border-sci-border">
          <EventLog />
        </div>
      </div>
    </main>
  </div>
</template>
