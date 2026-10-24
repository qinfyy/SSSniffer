<script setup>
import { ref, shallowRef, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import axios from 'axios'
import { protoRawDecode } from './proto_raw_decoder.js'
import { loadConfig } from './ConfigLoader.js'
import { formatTimestamp } from './formatTimestamp.js'
import Sidebar from './components/Sidebar.vue'
import FilterBar from './components/FilterBar.vue'
import PacketTable from './components/PacketTable.vue'
import PacketDetails from './components/PacketDetails.vue'
import SettingsModal from './components/SettingsModal.vue'

// 状态管理
const API_URL = ref('')
const configLoaded = ref(false)
const showDate = ref(true)

const clientID = ref('')

const getUrl = (path, extraParams = {}) => {
  const base = API_URL.value || window.location.origin
  const url = new URL(path, base)
  if (clientID.value) {
    url.searchParams.set('clientID', clientID.value)
  }

  for (const [key, value] of Object.entries(extraParams)) {
    url.searchParams.set(key, value)
  }

  return url.toString()
}

const sessionStarted = ref(false)
const currentPacket = ref(null)
const showSettings = ref(false)
const stick = ref(false)
const showDecode = ref(false)

// 主题相关
const isDarkTheme = ref(false)
const hasUserSetTheme = ref(false)

// 检测系统主题
const getSystemTheme = () => {
  return window.matchMedia('(prefers-color-scheme: dark)').matches
}

// 切换主题
const toggleTheme = () => {
  isDarkTheme.value = !isDarkTheme.value
  hasUserSetTheme.value = true
  applyTheme()
}

// 应用主题到 HTML 元素
const applyTheme = () => {
  if (isDarkTheme.value) {
    document.documentElement.classList.add('dark-theme')
  } else {
    document.documentElement.classList.remove('dark-theme')
  }
}

// 监听主题变化
watch(isDarkTheme, (newVal) => {
  if (hasUserSetTheme.value) {
    localStorage.setItem('theme', newVal ? 'dark' : 'light')
  }
})

// 过滤相关
const filterText = ref('')
const jsonFilter = ref('')
const useAnd = ref(true)

// 布局相关
const mainRef = ref(null)
const isResizing = ref(false)
const detailsWidthPercent = ref(30)

// 列宽相关
const columnWidths = ref({
  time: 210,
  idx: 40,
  src: 64,
  id: 48,
  name: 320,
  len: 48,
  json: 'auto'
})

// 数据包相关 - 改用 shallowRef 减少响应式开销
const packets = shallowRef([])
const packetIndex = ref(0)
const firstTime = ref(0)
const packetCounter = ref(0)
const scrollToIndex = ref(null)
const scrollToIndexFilter = ref(null)
const mainTableRef = ref(null)
const filterTableRef = ref(null)

// 批量更新优化 - 使用 requestAnimationFrame 合并同一帧的更新
const packetBuffer = ref([])
let rafId = null
let pendingFlush = false

// 设置相关
const settings = ref({
  startPort: '',
  endPort: '',
  useTcp: false,
  autoSavePcapFiles: true,
  autoSaveTextFiles: true,
  packetFilterList: []
})

// SSE 连接
let eventSource = null

const filteredPackets = computed(() => {
  if (!filterText.value && !jsonFilter.value) {
    return []
  }
  
  const packetsArray = packets.value
  const filterTextLower = filterText.value.toLowerCase()
  const jsonFilterLower = jsonFilter.value.toLowerCase()
  
  return packetsArray.filter(packet => {
    let and = filterText.value && jsonFilter.value
    if (!useAnd.value) and = false
    
    let text = false
    let json = false
    
    if (filterText.value) {
      if (packet.protoName?.toLowerCase().includes(filterTextLower)) {
        text = true
      }
      if (('' + packet.packetID).includes(filterText.value)) {
        text = true
      }
    }
    
    if (jsonFilter.value && packet.object) {
      // 缓存 JSON 字符串，避免重复计算
      if (!packet._jsonString) {
        packet._jsonString = JSON.stringify(packet.object).toLowerCase()
      }
      if (packet._jsonString.includes(jsonFilterLower)) {
        json = true
      }
    }
    
    if (!and && text) return true
    if (!and && json) return true
    if (and && text && json) return true
    
    return false
  })
})

// 配置加载
const initConfig = async () => {
  clientID.value = crypto.randomUUID()
  console.log('ClientID generated:', clientID.value)

  const config = await loadConfig('/Config.ini')
  API_URL.value = config?.Settings?.API || ''
  showDate.value = config?.Settings?.ShowDate?.toLowerCase() !== 'false'
  columnWidths.value = { ...columnWidths.value, time: showDate.value ? 210 : 120 }
  configLoaded.value = true
  console.log('Config loaded:', config)

  // 先注册
  await axios.get(getUrl('/api/registration'))
}

// 批量刷新数据包 - 使用 requestAnimationFrame 合并同一帧的更新
const flushPacketBuffer = () => {
  if (packetBuffer.value.length === 0) return
  
  // 合并当前数据包和缓冲区数据包
  const currentPackets = packets.value
  const newPackets = [...currentPackets, ...packetBuffer.value]
  
  // 使用 shallowRef 的 value 赋值，避免深度响应式
  packets.value = newPackets
  
  // 清空缓冲区
  packetBuffer.value = []
  
  nextTick(() => {
    // 通知主表更新可见区域
    if (mainTableRef.value?.updateVisibleRange) {
      mainTableRef.value.updateVisibleRange()
    }
    
    // 通知过滤表更新可见区域
    if (filterTableRef.value?.updateVisibleRange) {
      filterTableRef.value.updateVisibleRange()
    }
    
    // 自动滚动到底部
    if (stick.value && scrollToIndex.value) {
      scrollToIndex.value(packets.value.length - 1, { behavior: 'auto' })
    }
  })
}

// 调度刷新 - 使用 requestAnimationFrame 合并同一帧的多个包
const scheduleFlush = () => {
  if (pendingFlush) return
  pendingFlush = true
  
  rafId = requestAnimationFrame(() => {
    pendingFlush = false
    rafId = null
    flushPacketBuffer()
  })
}

// SSE 连接
const connectSSE = () => {
  if (eventSource) {
    eventSource.close()
  }
  
  eventSource = new EventSource(getUrl('/api/stream'))
  
  eventSource.addEventListener('open', (e) => {
    console.log('SSE connected', e)
  })
  
  eventSource.addEventListener('packetNotify', (e) => {
    const { time, fromServer, packetId, packetName, object, raw } = JSON.parse(e.data)
    
    packetCounter.value++

    console.log(time, fromServer, packetId, packetName, object, raw);
    
    if (packetId === 0) {
      packetIndex.value = 0
      firstTime.value = time
    }
    packetIndex.value++
    
    const newPacket = {
      globalId: packetCounter.value,
      time,
      displayTime: formatTimestamp(time, showDate.value),
      reltime: time - firstTime.value === 0 ? null : (time - firstTime.value) / 1000,
      index: packetIndex.value,
      packetID: packetId,
      protoName: packetName,
      source: !fromServer,
      object: object,
      packet: raw,
      decode: protoRawDecode(raw)
    }
    
    // 放入缓冲区
    packetBuffer.value.push(newPacket)
    
    // 使用 requestAnimationFrame 调度刷新，合并同一帧的多个包
    scheduleFlush()
  })
  
  eventSource.addEventListener('error', (e) => {
    console.error('SSE error', e)
    if (eventSource.readyState === EventSource.CLOSED) {
      setTimeout(connectSSE, 3000)
    }
  })
}

const disconnectSSE = () => {
  if (eventSource) {
    eventSource.close()
    eventSource = null
  }
  // 清空缓冲区
  if (rafId) cancelAnimationFrame(rafId)
  rafId = null
  pendingFlush = false
  flushPacketBuffer()
}

// API 方法
const startSession = async () => {
  try {
    await axios.get(getUrl('/api/start'))
    sessionStarted.value = true
  } catch (error) {
    console.error('Failed to start session', error)
  }
}

const stopSession = async () => {
  try {
    await axios.get(getUrl('/api/stop'))
    sessionStarted.value = false
  } catch (error) {
    console.error('Failed to stop session', error)
  }
}

const uploadFile = async (file) => {
  if (!file) return
  try {
    const formData = new FormData()
    formData.append('file', file)
    await axios.post(getUrl('/api/upload'), formData, {
      headers: { 'Content-Type': 'multipart/form-data' }
    })
  } catch (error) {
    console.error('Failed to upload file', error)
  }
}

const clearPackets = () => {
  packets.value = []
  packetBuffer.value = []
  packetCounter.value = 0
  packetIndex.value = 0
  firstTime.value = 0
  currentPacket.value = null
  columnWidths.value = {
    time: showDate.value ? 210 : 120,
    idx: 40,
    src: 64,
    id: 48,
    name: 320,
    len: 48,
    json: 'auto'
  }
}

// 选择数据包
const selectPacket = (packet, fromFilterTable = false) => {
  currentPacket.value = packet
  
  // 如果是从过滤表点击的，智能滚动到主表对应位置
  if (fromFilterTable && scrollToIndex.value) {
    // 在主表中找到对应的包索引
    const indexInPackets = packets.value.findIndex(p => p.globalId === packet.globalId)
    if (indexInPackets !== -1) {
      // 强制滚动到居中位置
      scrollToIndex.value(indexInPackets, { behavior: 'smooth', smart: false })
    }
  }
}

const openSettings = async () => {
  try {
    const response = await axios.get(getUrl('/api/GetConfig'))
    if (response.status === 200) {
      const data = response.data
      settings.value = {
        startPort: data.minKcpPort?.toString() || '',
        endPort: data.maxKcpPort?.toString() || '',
        useTcp: data.useTcp || false,
        autoSavePcapFiles: data.autoSavePcapFiles ?? true,
        autoSaveTextFiles: data.autoSaveTextFiles ?? true,
        packetFilterList: (data.packetFilter || []).filter(p => p.trim() !== '')
      }
    }
    showSettings.value = true
  } catch (error) {
    console.error('Failed to get config', error)
    showSettings.value = true
  }
}

const saveSettings = async (newSettings) => {
  if (!newSettings.startPort || !newSettings.endPort) {
    alert('Please fill in the complete port range')
    return
  }
  
  if (parseInt(newSettings.startPort) > parseInt(newSettings.endPort)) {
    alert('The start port cannot be larger than the end port')
    return
  }
  
  const filteredPacketFilter = newSettings.packetFilterList.filter(p => p.trim() !== '')
  const configData = {
    minKcpPort: parseInt(newSettings.startPort),
    maxKcpPort: parseInt(newSettings.endPort),
    packetFilter: filteredPacketFilter.length > 0 ? filteredPacketFilter : [''],
    autoSavePcapFiles: newSettings.autoSavePcapFiles,
    autoSaveTextFiles: newSettings.autoSaveTextFiles,
    useTcp: newSettings.useTcp
  }
  
  try {
    const response = await axios.post(getUrl('/api/SetConfig'), configData, {
      headers: { 'Content-Type': 'application/json' }
    })
    
    if (response.status === 200) {
      settings.value = { ...settings.value, ...newSettings }
      showSettings.value = false
    }
  } catch (error) {
    console.error('Failed to save settings', error)
    alert('Failed to save settings')
  }
}

const copyCurrentBin = () => {
  if (currentPacket.value?.packet) {
    navigator.clipboard.writeText(currentPacket.value.packet)
  }
}

const copyCurrentPacket = () => {
  if (currentPacket.value) {
    navigator.clipboard.writeText(JSON.stringify(currentPacket.value))
  }
}

// 拖拽调整
const startResize = (e) => {
  isResizing.value = true
  document.body.style.userSelect = 'none'
  document.body.style.cursor = 'col-resize'
}

const doResize = (e) => {
  if (!isResizing.value || !mainRef.value) return
  const rect = mainRef.value.getBoundingClientRect()
  const newWidth = ((rect.right - e.clientX) / rect.width) * 100
  detailsWidthPercent.value = Math.max(20, Math.min(50, newWidth))
}

const stopResize = () => {
  isResizing.value = false
  document.body.style.userSelect = ''
  document.body.style.cursor = ''
}

// 生命周期
onMounted(async () => {
  // 主题初始化逻辑
  const savedTheme = localStorage.getItem('theme')
  
  if (savedTheme) {
    // 用户之前手动设置过，使用保存的主题
    isDarkTheme.value = savedTheme === 'dark'
    hasUserSetTheme.value = true
    console.log('Theme loaded from localStorage:', savedTheme)
  } else {
    // 用户未设置过，自动检测系统主题
    isDarkTheme.value = getSystemTheme()
    hasUserSetTheme.value = false
    console.log('Theme detected from system:', isDarkTheme.value ? 'dark' : 'light')
  }
  
  // 应用主题
  applyTheme()
  
  // 监听系统主题变化（仅在用户未手动设置时生效）
  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
  const handleSystemThemeChange = (e) => {
    if (!hasUserSetTheme.value) {
      isDarkTheme.value = e.matches
      applyTheme()
      console.log('System theme changed:', isDarkTheme.value ? 'dark' : 'light')
    }
  }
  
  // 添加监听器
  mediaQuery.addEventListener('change', handleSystemThemeChange)
  
  // 清理监听器
  onUnmounted(() => {
    mediaQuery.removeEventListener('change', handleSystemThemeChange)
  })
  
  document.addEventListener('mousemove', doResize)
  document.addEventListener('mouseup', stopResize)
  await initConfig()
  connectSSE()
})

onUnmounted(() => {
  document.removeEventListener('mousemove', doResize)
  document.removeEventListener('mouseup', stopResize)
  disconnectSSE()
  if (rafId) cancelAnimationFrame(rafId)
})
</script>

<template>
  <div class="app-container">
    <div v-if="!configLoaded" class="loading-overlay">
      <div class="loading-content">
        <div class="loading-spinner"></div>
        <p>Loading configuration...</p>
      </div>
    </div>

    <Sidebar
      :session-started="sessionStarted"
      :current-packet="currentPacket"
      :has-filtered-packets="filteredPackets.length > 0"
      :stick="stick"
      :is-dark-theme="isDarkTheme"
      @toggle-session="sessionStarted ? stopSession() : startSession()"
      @clear="clearPackets"
      @open-settings="openSettings"
      @toggle-stick="stick = !stick"
      @toggle-theme="toggleTheme"
      @copy-bin="copyCurrentBin"
      @copy-packet="copyCurrentPacket"
      @upload="uploadFile"
    />

    <main ref="mainRef" class="main-host">
      <div class="main-content" :style="{ width: (100 - detailsWidthPercent) + '%' }">
        <FilterBar 
          v-model:filter="filterText"
          v-model:json-filter="jsonFilter"
          v-model:use-and="useAnd"
        />

        <div class="results-host" :class="{ open: filterText || jsonFilter }">
          <PacketTable
            ref="filterTableRef"
            :packets="filteredPackets"
            :current-packet="currentPacket"
            :column-widths="columnWidths"
            @update:column-widths="columnWidths = $event"
            @select-packet="(packet) => selectPacket(packet, true)"
            @scroll-to-index="(fn) => scrollToIndexFilter = fn"
          />
        </div>

        <div class="table-host">
          <PacketTable
            ref="mainTableRef"
            :packets="packets"
            :current-packet="currentPacket"
            :column-widths="columnWidths"
            @update:column-widths="columnWidths = $event"
            @select-packet="selectPacket"
            @scroll-to-index="(fn) => scrollToIndex = fn"
          />
        </div>
      </div>

      <div 
        class="resize" 
        :class="{ resizing: isResizing }"
        @mousedown="startResize"
      ></div>

      <PacketDetails
        ref="detailsRef"
        :packet="currentPacket"
        :show-decode="showDecode"
        :width="detailsWidthPercent"
        :is-dark-theme="isDarkTheme"
        @toggle-decode="showDecode = !showDecode"
      />
    </main>

    <SettingsModal
      v-if="showSettings"
      :settings="settings"
      @close="showSettings = false"
      @save="saveSettings"
    />
  </div>
</template>

<style scoped>
.app-container {
  display: flex;
  width: 100%;
  height: 100vh;
  overflow: hidden;
  position: relative;
  background: var(--bg-secondary);
  transition: background 0.3s ease;
}

.loading-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(255, 255, 255, 0.95);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.loading-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1rem;
  color: var(--text-secondary);
}

.loading-spinner {
  width: 40px;
  height: 40px;
  border: 3px solid var(--border-light);
  border-top-color: var(--primary-color);
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.main-host {
  display: flex;
  flex-grow: 1;
  overflow: hidden;
  position: relative;
  background: var(--bg-primary);
  transition: background 0.3s ease;
}

.main-content {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  flex-shrink: 0;
  transition: width 0.05s;
  overflow-x: hidden;
  background: var(--bg-primary);
}

.results-host {
  flex-grow: 1;
  max-height: 0%;
  background: var(--bg-tertiary);
  overflow: hidden;
  overflow-x: hidden;
  border-bottom: 1px solid var(--border-medium);
}

.results-host.open {
  max-height: 30%;
  min-height: 30%;
  border-bottom: 2px solid var(--primary-color);
}

.table-host {
  flex-grow: 1;
  overflow: hidden;
  overflow-x: hidden;
  background: var(--bg-primary);
}

.resize {
  position: relative;
  top: 0;
  height: 100%;
  width: 8px;
  margin-left: -4px;
  margin-right: -4px;
  background: var(--border-light);
  z-index: 10;
  cursor: col-resize;
  flex-shrink: 0;
  transition: background 0.2s;
}

.resize:hover,
.resize.resizing {
  background: var(--primary-color);
}

.resize::after {
  content: '';
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 2px;
  height: 30px;
  background: var(--border-dark);
  border-radius: 1px;
}

.resize:hover::after,
.resize.resizing::after {
  background: var(--text-light);
}
</style>
