<script setup lang="ts">
import { X } from 'lucide-vue-next'

defineProps<{
  title: string
  isOpen: boolean
  size?: 'sm' | 'md' | 'lg' | 'xl' | '2xl' | '4xl'
}>()

defineEmits<{
  (e: 'close'): void
}>()
</script>

<template>
  <Teleport to="body">
    <div v-if="isOpen" class="fixed inset-0 z-[9999] flex items-center justify-center p-4">
      <!-- Backdrop -->
      <div class="absolute inset-0 bg-black/50 backdrop-blur-sm" @click="$emit('close')"></div>
      
      <!-- Modal -->
      <div 
        class="relative bg-sci-panel rounded-lg border border-sci-border overflow-hidden animate-in fade-in zoom-in duration-200 flex flex-col max-h-[90vh]"
        :class="{
          'w-full max-w-sm': size === 'sm',
          'w-full max-w-md': !size || size === 'md',
          'w-full max-w-lg': size === 'lg',
          'w-full max-w-xl': size === 'xl',
          'w-full max-w-2xl': size === '2xl',
          'w-full max-w-4xl': size === '4xl'
        }"
      >
        <!-- Header -->
        <div class="flex items-center justify-between p-4 border-b border-sci-border bg-sci-panel-hover">
          <h3 class="font-display font-bold text-sci-text-pri tracking-wider flex items-center gap-2">
            <span class="w-1.5 h-4 bg-sci-cyan"></span>
            {{ title }}
          </h3>
          <button @click="$emit('close')" class="text-sci-text-dim hover:text-sci-danger transition-colors">
            <X class="w-5 h-5" />
          </button>
        </div>
        
        <!-- Content -->
        <div class="p-6 bg-sci-panel">
          <slot></slot>
        </div>
      </div>
    </div>
  </Teleport>
</template>
