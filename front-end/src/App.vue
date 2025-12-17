<script setup lang="ts">
import { RouterView } from 'vue-router'
import { useAppStore } from './stores/appStore'
import { storeToRefs } from 'pinia'
import { watch } from 'vue'

const store = useAppStore()
const { isCyberpunkMode } = storeToRefs(store)

// Apply theme class to body
watch(isCyberpunkMode, (val) => {
  if (val) {
    document.body.classList.add('theme-cyberpunk')
  } else {
    document.body.classList.remove('theme-cyberpunk')
  }
}, { immediate: true })

</script>

<template>
  <div id="app-root">
    <RouterView />
  </div>
</template>
