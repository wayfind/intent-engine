<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from '../composables/useI18n'

const props = defineProps<{
  parentId?: number | null
  parentName?: string
}>()

const emit = defineEmits<{
  (e: 'submit', data: { name: string, parentId: number | null, priority: number | null, spec: string }): void
  (e: 'cancel'): void
}>()

const { t } = useI18n()

const name = ref('')
const priority = ref<number | null>(null)
const spec = ref('')

const isValid = computed(() => name.value.trim().length > 0)

function submit() {
  if (!isValid.value) return
  emit('submit', {
    name: name.value,
    parentId: props.parentId || null,
    priority: priority.value,
    spec: spec.value
  })
}
</script>

<template>
  <div class="space-y-4">
    <!-- Parent Task (Readonly if present) -->
    <div v-if="props.parentId" class="space-y-1">
      <label class="block text-xs font-mono text-sci-text-dim">{{ t('PARENT_TASK') }}</label>
      <div class="w-full bg-sci-base border border-sci-border rounded-sm px-3 py-2 text-sm text-sci-text-sec font-mono">
        ID::{{ props.parentId.toString().padStart(4, '0') }} {{ props.parentName ? `(${props.parentName})` : '' }}
      </div>
    </div>

    <!-- Name -->
    <div class="space-y-1">
      <label class="block text-xs font-mono text-sci-text-dim">{{ t('TASK_NAME') }}</label>
      <input 
        v-model="name"
        @keyup.enter="submit"
        type="text" 
        class="w-full bg-sci-base border border-sci-border rounded-sm px-3 py-2 text-sm focus:outline-none focus:border-sci-cyan font-sans text-sci-text-pri placeholder-sci-text-dim"
        :placeholder="t('ENTER_NAME')"
        autofocus
      >
    </div>

    <!-- Priority -->
    <div class="space-y-1">
      <label class="block text-xs font-mono text-sci-text-dim">{{ t('PRIORITY') }}</label>
      <div class="flex gap-2">
        <button 
          v-for="opt in [
            { label: t('NONE'), value: null },
            { label: t('CRITICAL'), value: 1 },
            { label: t('HIGH'), value: 2 },
            { label: t('MEDIUM'), value: 3 },
            { label: t('LOW'), value: 4 }
          ]" 
          :key="opt.label"
          @click="priority = opt.value"
          class="px-3 py-1.5 text-xs font-mono border rounded-sm transition-all"
          :class="priority === opt.value 
            ? 'bg-sci-cyan text-sci-base border-sci-cyan' 
            : 'bg-sci-base border-sci-border text-sci-text-dim hover:border-sci-cyan hover:text-sci-cyan'"
        >
          {{ opt.label }}
        </button>
      </div>
    </div>

    <!-- Spec -->
    <div class="space-y-1">
      <label class="block text-xs font-mono text-sci-text-dim">{{ t('DESCRIPTION') }}</label>
      <textarea 
        v-model="spec"
        rows="12"
        class="w-full bg-sci-base border border-sci-border rounded-sm px-3 py-2 text-sm focus:outline-none focus:border-sci-cyan font-mono resize-none text-sci-text-sec placeholder-sci-text-dim scrollbar-thin scrollbar-thumb-sci-border scrollbar-track-transparent"
        :placeholder="t('ENTER_DESC')"
      ></textarea>
    </div>

    <!-- Actions -->
    <div class="flex justify-end gap-2 pt-4 border-t border-sci-border">
      <button 
        @click="$emit('cancel')"
        class="px-4 py-2 text-xs font-mono text-sci-text-dim hover:text-sci-text-pri transition-colors"
      >
        {{ t('CANCEL') }}
      </button>
      <button 
        @click="submit"
        class="px-6 py-2 text-xs font-mono bg-sci-cyan text-sci-base rounded-sm hover:bg-sci-cyan-dim hover:text-sci-cyan border border-transparent hover:border-sci-cyan transition-all disabled:opacity-50 disabled:cursor-not-allowed uppercase tracking-wider font-bold"
        :disabled="!isValid"
      >
        {{ t('CREATE') }}
      </button>
    </div>
  </div>
</template>
