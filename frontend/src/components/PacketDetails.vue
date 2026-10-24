<script setup>
import { ref, computed, watch } from 'vue'
import JsonEditorVue from 'json-editor-vue'
import 'vanilla-jsoneditor/themes/jse-theme-dark.css'

const props = defineProps({
  packet: Object,
  showDecode: Boolean,
  width: Number,
  isDarkTheme: Boolean
})

const emit = defineEmits(['toggleDecode'])

const objectEditorRef = ref(null)
const decodeEditorRef = ref(null)

const objectData = ref({})
const decodeData = ref({})

const showTwoEditors = computed(() => {
  return props.packet?.object && props.packet?.decode && props.showDecode
})

// 根据主题设置编辑器类名
const editorThemeClass = computed(() => {
  return props.isDarkTheme ? 'jse-theme-dark' : 'jse-theme-light'
})

watch(() => props.packet, (newPacket) => {
  if (newPacket?.object) {
    objectData.value = newPacket.object
  } else {
    objectData.value = {}
  }
  
  if (newPacket?.decode) {
    decodeData.value = newPacket.decode
  } else {
    decodeData.value = {}
  }
}, { immediate: true })

watch(() => props.showDecode, () => {
  if (props.packet?.decode) {
    decodeData.value = { ...props.packet.decode }
  }
})

const editorOptions = {
  mode: 'tree',
  mainMenuBar: true,
  navigationBar: true,
  statusBar: true,
  readOnly: true,
  indentation: 2
}
</script>

<template>
  <div class="details-host" :style="{ width: width + '%' }">
    <div v-if="!packet" class="no-selection">
      <p>Select a packet to view details</p>
    </div>

    <template v-else>
      <div :class="[showTwoEditors ? 'two-editor' : 'one-editor', editorThemeClass]">
        <div v-if="packet.object" class="editor-container">
          <!-- 悬浮控制按钮，融入菜单栏样式 -->
          <div class="overlay-controls">
            <button
              v-if="packet.decode"
              @click="emit('toggleDecode')"
              class="native-style-btn"
              :title="showDecode ? 'Hide Decode' : 'Show Decode'"
            >
              {{ showDecode ? 'HD' : 'RD' }}
            </button>
          </div>

          <div class="editor-wrapper">
            <JsonEditorVue
              ref="objectEditorRef"
              v-model="objectData"
              :mode="editorOptions.mode"
              :main-menu-bar="editorOptions.mainMenuBar"
              :navigation-bar="editorOptions.navigationBar"
              :status-bar="editorOptions.statusBar"
              :read-only="editorOptions.readOnly"
              :indentation="editorOptions.indentation"
              class="json-editor"
            />
          </div>
        </div>

        <div v-if="packet.decode && (showDecode || !packet.object)" class="editor-container">
          <div class="editor-wrapper">
            <JsonEditorVue
              ref="decodeEditorRef"
              v-model="decodeData"
              :mode="editorOptions.mode"
              :main-menu-bar="editorOptions.mainMenuBar"
              :navigation-bar="editorOptions.navigationBar"
              :status-bar="editorOptions.statusBar"
              :read-only="editorOptions.readOnly"
              :indentation="editorOptions.indentation"
              class="json-editor"
            />
          </div>
        </div>

      </div>
    </template>
  </div>
</template>

<style scoped>
.details-host {
  background: var(--bg-secondary);
  border-left: 1px solid var(--border-light);
  overflow-y: auto;
  overflow-x: hidden;
  max-height: 100%;
  flex-shrink: 0;
  transition: width 0.05s;
  display: flex;
  flex-direction: column;
}

.no-selection {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-tertiary);
}

/* 编辑器主题容器 */
.jse-theme-light {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
}

.jse-theme-dark {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
}

.two-editor .editor-container {
  height: 50%;
}

.one-editor .editor-container {
  height: 100%;
}

.editor-container {
  display: flex;
  flex-direction: column;
  border-bottom: 1px solid var(--border-light);
  overflow: hidden;
  position: relative;
}

.editor-container:last-child {
  border-bottom: none;
}

/* 悬浮控制栏 */
.overlay-controls {
  position: absolute;
  top: 0;
  right: 0;
  height: 36px;
  z-index: 10;
  pointer-events: none;
  display: flex;
  align-items: center;
}

/* 原生风格按钮 */
.native-style-btn {
  pointer-events: auto;
  background: transparent;
  border: none;
  border-left: 1px solid rgba(255, 255, 255, 0.15);
  color: #fff;
  padding: 0 18px;
  font-size: 0.8rem;
  font-weight: 550;
  cursor: pointer;
  transition: all 0.15s ease;
  height: 42px;
  line-height: 42px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  outline: none;
  letter-spacing: 0.3px;
}

/* 模仿原生按钮 */
.native-style-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  border-left-color: rgba(255, 255, 255, 0.3);
}

.native-style-btn:active {
  background: rgba(0, 0, 0, 0.15);
}

.editor-wrapper {
  flex-grow: 1;
  overflow: hidden;
  min-height: 0;
  overflow-x: hidden;
}

.json-editor {
  height: 100%;
  width: 100%;
}

/* 黑暗主题下调整按钮分隔线颜色 */
:deep(.jse-theme-dark .native-style-btn) {
  border-left-color: rgba(255, 255, 255, 0.2);
}

:deep(.jse-theme-dark .native-style-btn:hover) {
  background: rgba(255, 255, 255, 0.15);
}
</style>