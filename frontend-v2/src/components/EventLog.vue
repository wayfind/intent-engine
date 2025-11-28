<script setup lang="ts">
import { computed, ref } from 'vue'
import { useAppStore } from '../stores/appStore'
import { useI18n } from '../composables/useI18n'
import { MessageSquare, Send, Trash2 } from 'lucide-vue-next'
import InPlaceEditor from './InPlaceEditor.vue'
import MarkdownRenderer from './MarkdownRenderer.vue'
const { t } = useI18n()
const store = useAppStore()
const events = computed(() => store.events)
const newNote = ref('')

function getColor(type: string) {
  switch (type) {
    case 'decision': return 'text-sci-cyan border-sci-cyan bg-sci-cyan-dim'
    case 'blocker': return 'text-sci-danger border-sci-danger bg-sci-danger-dim'
    case 'milestone': return 'text-sci-success border-sci-success bg-sci-success-dim'
    default: return 'text-sci-text-sec border-sci-text-dim bg-sci-panel'
  }
}

function addNote() {
  if (!newNote.value.trim() || !store.viewingTaskId) return
  store.addEvent(store.viewingTaskId, 'note', newNote.value)
  newNote.value = ''
}

function updateEvent(id: number, newData: string) {
  if (!store.viewingTaskId) return
  store.updateEvent(store.viewingTaskId, id, newData)
}

function deleteEvent(id: number) {
  if (!store.viewingTaskId) return
  if (confirm(t('DELETE_LOG_CONFIRM'))) {
    store.deleteEvent(store.viewingTaskId, id)
  }
}
</script>

<template>
  <aside class="w-full bg-sci-panel flex flex-col h-full relative z-10">
    <!-- Integrated Header -->
    <div class="px-4 pt-4 pb-2">
      <h2 class="font-display font-bold text-sci-text-pri tracking-wider mb-1 flex items-center gap-2 text-sm">
        <MessageSquare class="w-4 h-4 text-sci-cyan" />
        {{ t('SYSTEM_LOGS') }}
      </h2>
    </div>

    <!-- Add Note Input (Seamless) -->
    <div v-if="store.viewingTaskId" class="px-4 pb-2">
      <div class="relative group">
        <input 
          v-model="newNote"
          @keyup.enter="addNote"
          type="text" 
          :placeholder="t('ADD_NOTE')" 
          class="w-full bg-transparent border border-transparent border-b-sci-border/50 rounded-none pl-0 pr-10 py-2 text-sm font-mono text-sci-text-pri focus:outline-none focus:border-b-sci-cyan transition-all placeholder-sci-text-dim/50 focus:bg-sci-base/30 focus:pl-2 focus:rounded-sm"
        >
        <button 
          @click="addNote"
          class="absolute right-2 top-2 text-sci-text-dim hover:text-sci-cyan transition-colors opacity-0 group-hover:opacity-100 focus:opacity-100"
          :disabled="!newNote.trim()"
        >
          <Send class="w-4 h-4" />
        </button>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto p-4 space-y-4 scrollbar-thin scrollbar-thumb-sci-border scrollbar-track-transparent">
      <div v-if="events.length === 0" class="text-center py-8 text-sci-text-dim font-mono text-xs opacity-50">
        {{ t('NO_DATA_STREAM') }}
      </div>
      
      <div 
        v-for="event in events" 
        :key="event.id"
        class="relative pl-4 border-l group transition-all hover:pl-5 pb-6"
        :class="event?.log_type ? getColor(event.log_type).split(' ')[1] : 'border-sci-border'"
      >
        <!-- Timeline Dot -->
        <div 
          class="absolute -left-[5px] top-1.5 w-2.5 h-2.5 rounded-full border-2 bg-sci-panel z-10 transition-colors"
          :class="event?.log_type ? getColor(event.log_type).split(' ')[1] : 'border-sci-border'"
        ></div>

        <!-- Header: Time + Type + Actions -->
        <div class="flex items-center gap-2 mb-1" v-if="event?.log_type">
          <!-- Inline Time -->
          <span class="text-[10px] font-mono text-sci-text-dim opacity-60">
            {{ new Date(event.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) }}
          </span>

          <span class="text-xs font-mono font-bold uppercase tracking-wider opacity-90" :class="getColor(event.log_type).split(' ')[0]">
            {{ t(`TYPE_${event.log_type.toUpperCase()}` as any) || event.log_type }}
          </span>
          
          <!-- Delete Action -->
          <button 
            @click="deleteEvent(event.id)"
            class="ml-auto opacity-0 group-hover:opacity-100 hover:text-sci-danger transition-opacity"
            :title="t('DELETE_LOG')"
          >
            <Trash2 class="w-3 h-3" />
          </button>
        </div>
        
        <!-- Clean Content Area -->
        <div class="text-sm text-sci-text-pri leading-relaxed w-full">
          <InPlaceEditor 
            :model-value="event.discussion_data" 
            multiline 
            markdown
            @save="(val) => updateEvent(event.id, val)"
          >
            <template #display>
              <MarkdownRenderer 
                :source="event.discussion_data" 
                small
              />
            </template>
          </InPlaceEditor>
        </div>
      </div>
    </div>
  </aside>
</template>
