<script setup lang="ts">
import { computed } from 'vue'
import { useAppStore } from '../stores/appStore'
import { Target, Play, Square } from 'lucide-vue-next'

interface TaskNode {
  id: number
  name: string
  status: string
  children?: TaskNode[]
  isOpen?: boolean
  [key: string]: any
}

const props = defineProps<{
  node: TaskNode
  depth?: number
  forceOpen?: boolean
}>()

const store = useAppStore()
const depth = props.depth || 0
const paddingLeft = computed(() => `${depth * 12 + 8}px`)

const isActive = computed(() => store.viewingTaskId === props.node.id)

function selectTask() {
  store.viewingTaskId = props.node.id
  store.fetchTaskDetail(props.node.id)
}

const activeClass = computed(() => {
  if (!isActive.value) {
    return 'border-transparent hover:bg-sci-panel-hover hover:text-sci-text-pri'
  }
  
  switch (props.node.priority) {
    case 1: return 'bg-sci-danger-dim border-sci-danger text-sci-danger'
    case 2: return 'bg-sci-orange-dim border-sci-orange text-sci-orange'
    case 3: return 'bg-sci-cyan-dim border-sci-cyan text-sci-cyan'
    default: return 'bg-sci-panel-hover border-sci-text-dim text-sci-text-pri'
  }
})
</script>

<template>
  <div class="select-none relative">
    <!-- Connecting Line (Vertical) -->
    <div v-if="depth > 0" class="absolute left-0 top-0 bottom-0 w-px bg-sci-border opacity-30" :style="{ left: `${(depth * 12) - 6}px` }"></div>

    <div 
      class="group flex items-center gap-2 py-1.5 px-2 cursor-pointer transition-all border-l-2 relative overflow-hidden"
      :class="activeClass"
      :style="{ paddingLeft }"
      @click="selectTask"
    >


      <!-- Status Indicator -->
      <div class="relative flex items-center justify-center w-5 h-5">
        <Play v-if="props.node.status === 'doing'" class="w-[18px] h-[18px] text-sci-text-dim" />
        
        <div v-else-if="props.node.status === 'done'" class="relative w-[18px] h-[18px]">
          <Square class="w-full h-full text-sci-text-dim" />
          <span class="absolute -top-[2px] right-0 flex items-center justify-center text-sci-text-dim font-serif font-bold text-lg leading-none select-none">✓</span>
        </div>
        
        <Square v-else class="w-[18px] h-[18px] text-sci-text-dim" />
      </div>
      
      <!-- Task ID -->
      <span class="text-[10px] font-mono text-sci-text-dim opacity-60 shrink-0">
        #{{ props.node.id }}
      </span>

      <span
        class="text-xs font-mono truncate flex-1 z-10 transition-colors"
        :class="[
          props.node.priority === 1 ? 'text-sci-danger' : '',
          props.node.priority === 2 ? 'text-sci-orange' : '',
          props.node.priority === 3 ? 'text-sci-cyan' : '',
          isActive && (!props.node.priority || props.node.priority > 3) ? 'text-sci-text-pri' : '',
          !isActive && (!props.node.priority || props.node.priority > 3) ? 'text-sci-text-sec' : ''
        ]"
      >
        {{ props.node.name }}
      </span>
      
      <!-- Current Focus Indicator -->
      <Target v-if="store.currentTaskId === props.node.id" class="w-[18px] h-[18px] text-sci-orange animate-pulse" />
    </div>
    
    <div v-if="props.node.children?.length" class="relative">
      <TaskTreeItem 
        v-for="child in props.node.children" 
        :key="child.id" 
        :node="child" 
        :depth="depth + 1" 
        :force-open="forceOpen"
      />
    </div>
  </div>
</template>
