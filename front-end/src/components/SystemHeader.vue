<script setup lang="ts">
import { useAppStore } from '../stores/appStore'
import { Folder, Cpu, Globe, Sun, Moon } from 'lucide-vue-next'
import { useI18n } from '../composables/useI18n'

const store = useAppStore()
const { currentLocale, toggleLocale } = useI18n()

function toggleTheme() {
  store.toggleTheme()
}
</script>

<template>
  <header class="h-12 bg-sci-panel border-b border-sci-border flex items-end justify-between px-4 select-none">
    <!-- Left: Logo & Title -->
    <div class="flex items-end gap-4">
      <div class="flex items-center gap-2 text-sci-cyan pb-2">
        <Cpu class="w-6 h-6" />
        <span class="font-display font-bold text-lg tracking-wider">INTENT ENGINE CONSOLE</span>
      </div>
      
      <div class="h-6 w-px bg-sci-border mx-2"></div>
      
      <!-- Project Modules (Scrollable Tabs) -->
      <div class="flex items-end gap-1 overflow-x-auto no-scrollbar -mb-[1px] pt-2">
        <button 
          v-for="proj in store.projects" 
          :key="proj.path"
          @click="store.switchProject(proj.path)"
          class="flex items-center gap-2 px-4 py-2 rounded-t-sm border-t border-x transition-all text-xs font-mono uppercase tracking-wide whitespace-nowrap relative"
          :class="[
            store.currentProject?.path === proj.path 
              ? 'bg-sci-base border-x-sci-border border-t-sci-cyan text-sci-cyan border-b-transparent z-10 font-bold' 
              : 'bg-transparent border-t-transparent border-x-transparent text-sci-text-muted hover:bg-white/5 hover:text-sci-text-main border-b-transparent'
          ]"
        >
          <!-- Bottom cover for active tab -->
          <div v-if="store.currentProject?.path === proj.path" class="absolute -bottom-px left-0 right-0 h-px bg-sci-base"></div>
          
          <Folder class="w-3.5 h-3.5" />
          <span>{{ proj.name }}</span>
          <div class="w-1.5 h-1.5 rounded-full" :class="proj.is_online ? 'bg-sci-success' : 'bg-sci-text-dim'"></div>
          
          <!-- Close button for offline tabs -->
          <div 
            v-if="!proj.is_online"
            @click.stop="store.removeProject(proj.path)"
            class="ml-1 p-0.5 rounded hover:bg-white/20 text-sci-text-dim hover:text-white transition-colors"
            title="Remove Project"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
          </div>
        </button>
      </div>
    </div>

    <!-- Right: System Controls -->
    <div class="flex items-center gap-4 text-xs font-mono text-sci-text-muted pb-3">
      <div class="text-sci-text-dim">v2.0.1</div>
      
      <div class="h-4 w-px bg-sci-border"></div>

      <!-- Language Switcher -->
      <button 
        @click="toggleLocale"
        class="flex items-center gap-1 hover:text-sci-cyan transition-colors uppercase"
        title="Switch Language"
      >
        <Globe class="w-4 h-4" />
        <span>{{ currentLocale === 'en' ? 'EN' : 'CN' }}</span>
      </button>

      <!-- Theme Toggle -->
      <button 
        @click="toggleTheme"
        class="flex items-center gap-1 hover:text-sci-cyan transition-colors"
        :title="store.isCyberpunkMode ? 'Switch to Light Mode' : 'Switch to Cyberpunk Mode'"
      >
        <Sun v-if="store.isCyberpunkMode" class="w-4 h-4" />
        <Moon v-else class="w-4 h-4" />
      </button>
    </div>
  </header>
</template>
