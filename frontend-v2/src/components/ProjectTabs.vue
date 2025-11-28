<script setup lang="ts">
import { useAppStore } from '../stores/appStore'
import { Folder } from 'lucide-vue-next'

const store = useAppStore()
</script>

<template>
  <div class="h-10 flex items-center gap-2 overflow-x-auto px-4 border-b border-sci-border bg-sci-panel flex-shrink-0">
    <button 
      v-for="proj in store.projects" 
      :key="proj.path"
      @click="store.switchProject(proj.path)"
      class="flex-shrink-0 flex items-center gap-2 px-3 py-1.5 text-xs font-mono border-t border-x rounded-t-md transition-all whitespace-nowrap relative top-[1px]"
      :class="[
        store.currentProject?.path === proj.path 
          ? 'bg-sci-base border-sci-border border-b-sci-base text-sci-cyan font-medium z-10' 
          : 'bg-slate-50 border-transparent hover:bg-slate-100 text-slate-500'
      ]"
    >
      <Folder class="w-3.5 h-3.5" :class="store.currentProject?.path === proj.path ? 'text-sci-cyan' : 'text-slate-400'" />
      <span>{{ proj.name }}</span>
      
      <!-- Status Indicator -->
      <div class="relative flex h-1.5 w-1.5 ml-1">
        <span v-if="proj.is_online" class="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75"></span>
        <span class="relative inline-flex rounded-full h-1.5 w-1.5" :class="proj.is_online ? 'bg-green-500' : 'bg-slate-300'"></span>
      </div>
    </button>
  </div>
</template>
