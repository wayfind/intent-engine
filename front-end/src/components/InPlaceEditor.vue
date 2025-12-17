<script setup lang="ts">
import { ref, nextTick, watch } from 'vue'
import { Check, X } from 'lucide-vue-next'

const props = defineProps<{
  modelValue: string | null
  multiline?: boolean
  placeholder?: string
  markdown?: boolean
}>()

const emit = defineEmits(['update:modelValue', 'save', 'cancel'])

const isEditing = ref(false)
const editValue = ref('')
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const inputRef = ref<HTMLInputElement | null>(null)

// Watch for external data changes - cancel editing when modelValue changes
// This handles the case where user switches to a different task while editing
watch(() => props.modelValue, () => {
  if (isEditing.value) {
    isEditing.value = false
  }
})

function startEditing() {
  editValue.value = props.modelValue || ''
  isEditing.value = true
  nextTick(() => {
    if (props.multiline) {
      if (textareaRef.value) {
        textareaRef.value.focus()
        adjustHeight()
      }
    } else {
      if (inputRef.value) {
        inputRef.value.focus()
      }
    }
  })
}

function save() {
  emit('update:modelValue', editValue.value)
  emit('save', editValue.value)
  isEditing.value = false
}

function cancel() {
  isEditing.value = false
  emit('cancel')
}

function adjustHeight() {
  if (!textareaRef.value) return
  textareaRef.value.style.height = 'auto'
  textareaRef.value.style.height = textareaRef.value.scrollHeight + 'px'
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
    save()
  } else if (e.key === 'Escape') {
    cancel()
  }
}
</script>

<template>
  <div class="relative group">
    <!-- Display Mode -->
    <div 
      v-show="!isEditing" 
      @click="startEditing"
      class="cursor-pointer border border-transparent rounded p-1 transition-all min-h-[1.5em]"
      :class="{
        'italic text-sci-text-dim': !modelValue,
        'hover:bg-sci-panel-hover': true,
        'hover:border-sci-border': true,
        'bg-sci-base': markdown
      }"
    >
      <slot name="display">
        <div v-if="markdown" class="prose prose-sm max-w-none pointer-events-none" v-html="modelValue || placeholder"></div>
        <div v-else>{{ modelValue || placeholder }}</div>
      </slot>
      
      <!-- Edit Hint -->
      <div class="absolute right-1 bottom-1 opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none">
        <span class="text-[10px] font-mono text-sci-text-dim bg-sci-base border border-sci-border px-1 rounded">CLICK TO EDIT</span>
      </div>
    </div>

    <!-- Edit Mode -->
    <div v-show="isEditing" class="relative">
      <textarea
        v-if="multiline"
        ref="textareaRef"
        v-model="editValue"
        @input="adjustHeight"
        @keydown="handleKeydown"
        class="w-full bg-sci-base border border-sci-cyan border-b-0 rounded-t p-2 focus:outline-none focus:ring-0 font-mono text-sm resize-none block text-sci-text-pri"
        :placeholder="placeholder"
        rows="3"
      ></textarea>
      <input
        v-else
        ref="inputRef"
        v-model="editValue"
        @keydown.enter="save"
        @keydown.esc="cancel"
        class="w-full bg-sci-base border border-sci-cyan border-b-0 rounded-t px-2 py-1 focus:outline-none focus:ring-0 font-sans text-sci-text-pri"
        :placeholder="placeholder"
      >
      
      <!-- Footer Toolbar (Unified) -->
      <div class="flex items-center justify-between bg-sci-base border border-sci-cyan border-t-0 rounded-b px-2 py-1">
        <!-- Hint -->
        <div class="flex items-center gap-2 text-[10px] text-sci-text-dim font-mono">
          <span class="px-1.5 py-px border border-sci-border rounded bg-sci-panel leading-none">
            {{ multiline ? 'Ctrl+Enter' : 'Enter' }}
          </span>
          <span class="leading-none">to save</span>
        </div>

        <!-- Actions -->
        <div class="flex items-center gap-2">
          <button 
            @click="cancel"
            class="text-xs font-mono text-sci-text-dim hover:text-sci-text-pri px-2 py-0.5 rounded hover:bg-sci-panel-hover transition-colors flex items-center gap-1"
            title="Cancel (Esc)"
          >
            <X class="w-3 h-3" />
            <span>Cancel</span>
          </button>
          <button 
            @click="save"
            class="text-xs font-mono bg-sci-cyan text-sci-base px-3 py-0.5 rounded hover:opacity-90 transition-opacity flex items-center gap-1 font-bold"
            :title="multiline ? 'Save (Ctrl+Enter)' : 'Save (Enter)'"
          >
            <Check class="w-3 h-3" />
            <span>Save</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
