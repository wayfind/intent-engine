<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { marked } from 'marked'
import markedKatex from 'marked-katex-extension'
import hljs from 'highlight.js'
import DOMPurify from 'dompurify'
import mermaid from 'mermaid'
import * as echarts from 'echarts'
import { useAppStore } from '@/stores/appStore'

// Import Styles
import 'highlight.js/styles/github.css' // Light theme
import 'katex/dist/katex.min.css'

const props = defineProps<{
  source?: string | null
  placeholder?: string
  small?: boolean
}>()

const store = useAppStore()

const renderedContent = ref('')
const isLoading = ref(false)
const containerRef = ref<HTMLElement | null>(null)
let isMounted = false
let charts: echarts.ECharts[] = []

// Custom Light Palette (Softer/Brighter)
const lightPalette = [
  '#5470c6', '#91cc75', '#fac858', '#ee6666', '#73c0de', '#3ba272', '#fc8452', '#9a60b4', '#ea7ccc'
]

// Initialize Mermaid
mermaid.initialize({ 
  startOnLoad: false,
  theme: 'neutral', // Light/Neutral theme
  securityLevel: 'loose',
  fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace'
})

// Configure Marked
const renderer = new marked.Renderer()

renderer.code = ({ text, lang }: { text: string, lang?: string }) => {
  // Mermaid
  if (lang === 'mermaid') {
    return `<div class="mermaid">${text}</div>`
  }
  // Echarts
  if (lang === 'echarts') {
    return `<div class="echarts-container w-full h-[300px]" data-option="${encodeURIComponent(text)}"></div>`
  }
  
  // Syntax Highlighting
  const language = (lang && hljs.getLanguage(lang)) ? lang : 'plaintext'
  try {
    const highlighted = hljs.highlight(text, { language }).value
    return `<pre><code class="hljs language-${language}">${highlighted}</code></pre>`
  } catch (e) {
    return `<pre><code class="hljs language-${language}">${text}</code></pre>`
  }
}

marked.use(
  { renderer },
  markedKatex({
    throwOnError: false
  })
)

onMounted(() => {
  isMounted = true
})

onUnmounted(() => {
  isMounted = false
  disposeCharts()
})

let resizeObservers: ResizeObserver[] = []

function disposeCharts() {
  resizeObservers.forEach(ro => ro.disconnect())
  resizeObservers = []
  charts.forEach(c => c.dispose())
  charts = []
}

function migrateOptions(option: any) {
  if (!option || typeof option !== 'object') return

  // Fix itemStyle.emphasis -> emphasis.itemStyle
  if (option.itemStyle && option.itemStyle.emphasis) {
    if (!option.emphasis) option.emphasis = {}
    option.emphasis.itemStyle = option.itemStyle.emphasis
    delete option.itemStyle.emphasis
  }

  // Recursive check for children (series, etc.)
  for (const key in option) {
    if (Array.isArray(option[key])) {
      option[key].forEach((item: any) => migrateOptions(item))
    } else if (typeof option[key] === 'object') {
      migrateOptions(option[key])
    }
  }
}

async function renderDiagrams() {
  if (!containerRef.value) return

  // Render Mermaid
  try {
    await mermaid.run({
      nodes: containerRef.value.querySelectorAll('.mermaid')
    })
  } catch (e) {
    console.error('Mermaid render failed:', e)
  }

  // Render Echarts
  disposeCharts()
  const echartsContainers = containerRef.value.querySelectorAll('.echarts-container')
  echartsContainers.forEach((el) => {
    try {
      const optionStr = decodeURIComponent(el.getAttribute('data-option') || '')
      const option = JSON.parse(optionStr)
      
      // Migrate deprecated options
      migrateOptions(option)
      
      // Inject system font family
      if (!option.textStyle) option.textStyle = {}
      option.textStyle.fontFamily = 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Microsoft YaHei", "PingFang SC", "Hiragino Sans GB", "Heiti SC", monospace'
      
      // Apply Theme based on Store
      const isDark = store.isCyberpunkMode
      
      // Force background to be transparent or match theme
      if (!option.backgroundColor) {
        option.backgroundColor = 'transparent'
      }

      // If Light Mode, ensure we use a good palette if not specified
      if (!isDark && !option.color) {
        option.color = lightPalette
      }

      // Initialize with theme
      const theme = isDark ? 'dark' : undefined
      const chart = echarts.init(el as HTMLElement, theme, { renderer: 'svg' })
      
      chart.setOption(option)
      charts.push(chart)
      
      const ro = new ResizeObserver(() => {
        if (!chart.isDisposed()) {
          chart.resize()
        }
      })
      ro.observe(el)
      resizeObservers.push(ro)
    } catch (e) {
      console.error('Echarts render failed:', e)
      el.innerHTML = `<div class="text-sci-danger text-xs font-mono p-2 border border-sci-danger rounded">Echarts Error: Invalid JSON</div>`
    }
  })
}

// Watch for theme changes to re-render
watch(() => store.isCyberpunkMode, () => {
  renderDiagrams()
})


watch(() => props.source, async (newSource) => {
  if (!newSource) {
    renderedContent.value = ''
    return
  }
  
  isLoading.value = true
  try {
    const result = await marked.parse(newSource)
    if (!isMounted) return
    
    if (typeof result === 'string') {
        renderedContent.value = DOMPurify.sanitize(result, {
          ADD_TAGS: ['div', 'input', 'video', 'audio', 'iframe', 'source'], // Allow containers, checkboxes, and media
          ADD_ATTR: ['class', 'data-option', 'style', 'type', 'checked', 'disabled', 'src', 'controls', 'width', 'height', 'allow', 'allowfullscreen']
        })
        
        // Render diagrams after DOM update
        nextTick(() => {
          renderDiagrams()
        })
    } else {
        console.error('marked.parse returned non-string:', result)
        renderedContent.value = newSource
    }
  } catch (e) {
    console.error('Markdown parsing failed:', e)
    if (isMounted) renderedContent.value = newSource // Fallback
  } finally {
    if (isMounted) isLoading.value = false
  }
}, { immediate: true })
</script>

<template>
  <div 
    ref="containerRef"
    class="prose max-w-none text-sci-text-sec break-words relative min-h-[1em]"
    :class="[
      small ? 'prose-xs' : 'prose-sm'
    ]"
  >
    <!-- Loading Indicator -->
    <div v-if="isLoading" class="absolute inset-0 bg-sci-base/50 flex items-center justify-center z-10">
      <div class="w-4 h-4 border-2 border-sci-cyan border-t-transparent rounded-full animate-spin"></div>
    </div>

    <!-- Rendered Content -->
    <div v-show="renderedContent" v-html="renderedContent"></div>
    
    <!-- Fallback: Raw Source (if render failed but source exists) -->
    <div v-show="!renderedContent && source && !isLoading" class="whitespace-pre-wrap font-mono text-xs text-sci-danger">
      [RENDER FAILED]
      {{ source }}
    </div>

    <!-- Placeholder -->
    <div v-show="!renderedContent && !source && placeholder" class="text-sci-text-dim italic">{{ placeholder }}</div>
    
    <!-- Spacer -->
    <div v-show="!renderedContent && !source && !placeholder" class="opacity-0">.</div>
  </div>
</template>

<style>
/* Custom Font Stack for Code Blocks to support Chinese */
.prose pre, .prose code {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace, "Microsoft YaHei", "PingFang SC", "Hiragino Sans GB", "Heiti SC" !important;
}

/* Ensure code blocks have a light background and proper padding */
.prose pre {
  background-color: #f6f8fa !important; /* GitHub Light background */
  border: 1px solid #e1e4e8;
  border-radius: 0.375rem;
  padding: 1em;
  margin-top: 1em;
  margin-bottom: 1em;
  overflow-x: auto;
  color: #24292e; /* GitHub text color */
}

/* Adjust font size for better readability */
.prose code {
  font-size: 0.9em;
}

/* Explicitly scale down headers in small mode (EventLog) */
.prose-xs h1 { font-size: 1.1em; margin-top: 0.8em; margin-bottom: 0.4em; }
.prose-xs h2 { font-size: 1em; margin-top: 0.8em; margin-bottom: 0.4em; }
.prose-xs h3 { font-size: 0.9em; margin-top: 0.6em; margin-bottom: 0.3em; }
.prose-xs h4 { font-size: 0.85em; margin-top: 0.6em; margin-bottom: 0.3em; }
.prose-xs p { margin-top: 0.5em; margin-bottom: 0.5em; }
.prose-xs ul, .prose-xs ol { margin-top: 0.5em; margin-bottom: 0.5em; }
.prose-xs li { margin-top: 0.2em; margin-bottom: 0.2em; }
</style>
