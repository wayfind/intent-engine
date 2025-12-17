<script setup lang="ts">
import { ref, computed } from 'vue'
import { Code, ChevronDown, Copy, Check } from 'lucide-vue-next'

const props = defineProps<{
  taskId: number
}>()

const isOpen = ref(false)
const copyStatus = ref<'idle' | 'success'>('idle')
let dropdownTimer: any = null

// Only show debug menu when accessing via port 1393 (dev mode)
const isDebugMode = computed(() => {
  return window.location.port === '1393'
})

function openDropdown() {
  if (dropdownTimer) clearTimeout(dropdownTimer)
  isOpen.value = true
}

function closeDropdown() {
  dropdownTimer = setTimeout(() => {
    isOpen.value = false
  }, 150)
}

async function copyToClipboard(text: string) {
  try {
    await navigator.clipboard.writeText(text)
    copyStatus.value = 'success'
    setTimeout(() => {
      copyStatus.value = 'idle'
    }, 2000)
  } catch (e) {
    console.error('Failed to copy:', e)
  }
}

async function onGetTask() {
  try {
    const res = await fetch(`/api/tasks/${props.taskId}?with_events=true`)
    const json = await res.json()
    await copyToClipboard(JSON.stringify(json.data, null, 2))
    closeDropdown()
  } catch (e) {
    console.error('Failed to fetch task:', e)
  }
}

async function onGetContext() {
  try {
    const res = await fetch(`/api/tasks/${props.taskId}/context`)
    const json = await res.json()
    await copyToClipboard(JSON.stringify(json.data, null, 2))
    closeDropdown()
  } catch (e) {
    console.error('Failed to fetch context:', e)
  }
}
</script>

<template>
  <div v-if="isDebugMode" class="relative" @mouseenter="openDropdown" @mouseleave="closeDropdown">
    <button
      class="flex items-center gap-2 bg-sci-base border border-sci-border rounded-sm px-3 h-7 hover:bg-sci-panel-hover hover:border-sci-text-dim hover:text-sci-text-pri transition-all font-mono text-xs"
      :class="{ 'text-sci-cyan': copyStatus === 'success' }"
    >
      <component :is="copyStatus === 'success' ? Check : Code" class="w-4 h-4" />
      <span class="uppercase tracking-wider">Debug</span>
      <ChevronDown class="w-3 h-3 opacity-30" />
    </button>

    <div v-if="isOpen" class="absolute right-0 mt-1 w-48 bg-sci-base border border-sci-border shadow-xl rounded-sm z-50 py-1 backdrop-blur-sm flex flex-col">
      <button
        @click="onGetTask"
        class="w-full flex items-center gap-2 px-3 py-2 text-xs font-mono text-left hover:bg-sci-panel-hover hover:text-sci-cyan transition-colors"
      >
        <Copy class="w-3.5 h-3.5" />
        Get (task_get)
      </button>
      <div class="h-px bg-sci-border my-1 opacity-50"></div>
      <button
        @click="onGetContext"
        class="w-full flex items-center gap-2 px-3 py-2 text-xs font-mono text-left hover:bg-sci-panel-hover hover:text-sci-cyan transition-colors"
      >
        <Copy class="w-3.5 h-3.5" />
        Context (task_context)
      </button>
    </div>
  </div>
</template>
