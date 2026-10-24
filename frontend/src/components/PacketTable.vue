<script setup>
import { ref, watch, onMounted, onUnmounted, nextTick, computed, shallowRef, markRaw } from 'vue'

const props = defineProps({
  packets: Array,
  currentPacket: Object,
  columnWidths: Object
})

const emit = defineEmits(['selectPacket', 'scrollToIndex', 'update:columnWidths'])

const tbodyRef = ref(null)
const headerRef = ref(null)
const isResizing = ref(false)
const resizingColumn = ref(null)
const resizeStartX = ref(0)
const resizeStartWidth = ref(0)

// 列定义
const columns = [
  { key: 'time', label: 'TIME' },
  { key: 'idx', label: '#' },
  { key: 'src', label: 'SENDER' },
  { key: 'id', label: 'ID' },
  { key: 'name', label: 'PROTO NAME' },
  { key: 'len', label: 'LENGTH' },
  { key: 'json', label: 'JSON' }
]

// 默认列宽
const defaultColumnWidths = {
  time: 210,
  idx: 50,
  src: 80,
  id: 60,
  name: 200,
  len: 70,
  json: 200
}

// 各列最小宽度
const minColumnWidths = {
  time: 120,
  idx: 40,
  src: 60,
  id: 50,
  name: 100,
  len: 50,
  json: 100
}

// 当前列宽
const columnWidths = ref({ ...defaultColumnWidths })

// 虚拟滚动相关
const ROW_HEIGHT = 34
const VISIBLE_OVERSCAN = 10
const scrollTop = ref(0)
const visibleStart = ref(0)
const visibleEnd = ref(0)
const totalHeight = ref(0)

// 列宽计算优化 - 记录每列当前最大宽度值，增量计算
const columnMaxValues = shallowRef({
  time: 0,
  idx: 0,
  src: 0,
  id: 0,
  name: 0,
  len: 0
})

// 测量文字宽度 - 缓存 canvas 上下文
const measureCanvas = typeof window !== 'undefined' ? document.createElement('canvas') : null
const measureContext = measureCanvas ? measureCanvas.getContext('2d') : null
const measureCache = new Map()

const measureTextWidth = (text, fontSize = 12, fontWeight = '600') => {
  if (!measureContext) return 0
  
  const cacheKey = `${text}_${fontSize}_${fontWeight}`
  if (measureCache.has(cacheKey)) {
    return measureCache.get(cacheKey)
  }
  
  measureContext.font = `${fontWeight} ${fontSize}px 'Inter', system-ui, sans-serif`
  const width = measureContext.measureText(text).width
  measureCache.set(cacheKey, width)
  
  // 限制缓存大小
  if (measureCache.size > 1000) {
    const firstKey = measureCache.keys().next().value
    measureCache.delete(firstKey)
  }
  
  return width
}

// 获取列值的文本表示
const getCellValue = (packet, columnKey) => {
  switch (columnKey) {
    case 'time':
      return packet.displayTime || ''
    case 'idx':
      return (packet.index || 0).toString()
    case 'src':
      return packet.source ? 'CLIENT' : 'SERVER'
    case 'id':
      return (packet.packetID || 0).toString()
    case 'name':
      return packet.protoName || ''
    case 'len':
      return (packet.packet?.length || 0).toString()
    default:
      return ''
  }
}

// 计算列宽需要的像素值
const getTextWidthForColumn = (columnKey, text) => {
  const padding = 24
  const maxLimits = { time: 240, idx: 80, id: 100, name: 400, len: 80 }
  
  const width = measureTextWidth(text, 12, columnKey === 'id' || columnKey === 'len' ? '700' : '400') + padding
  return Math.min(width, maxLimits[columnKey] || 400)
}

// 计算单列宽度 - 增量计算，只检查新包
const calculateColumnWidth = (columnKey, packetsToCheck, currentWidth) => {
  if (columnKey === 'json') return currentWidth
  
  let maxWidth = currentWidth
  const currentMax = columnMaxValues.value[columnKey] || 0
  
  // 计算表头宽度
  const column = columns.find(c => c.key === columnKey)
  if (column) {
    const headerWidth = measureTextWidth(column.label, 11, '700') + 24
    maxWidth = Math.max(maxWidth, headerWidth)
  }
  
  // 增量计算：只检查可能超出当前宽度的包
  let newMax = currentMax
  for (const packet of packetsToCheck) {
    const text = getCellValue(packet, columnKey)
    const width = getTextWidthForColumn(columnKey, text)
    
    if (width > newMax) {
      newMax = width
      maxWidth = Math.max(maxWidth, width)
    }
  }
  
  // 更新记录的最大值
  if (newMax > currentMax) {
    columnMaxValues.value = { ...columnMaxValues.value, [columnKey]: newMax }
  }
  
  // 确保不小于最小宽度
  const minWidth = minColumnWidths[columnKey] || 40
  return Math.max(maxWidth, minWidth)
}

// 计算列宽
let calculateTimeout = null
const calculateColumnWidths = (forceAll = false) => {
  if (calculateTimeout) {
    clearTimeout(calculateTimeout)
  }
  
  calculateTimeout = setTimeout(() => {
    const packets = props.packets || []
    const totalPackets = packets.length
    
    let packetsToCheck = packets
    if (totalPackets >= 1000 && !forceAll && visibleEnd.value > visibleStart.value) {
      packetsToCheck = packets.slice(visibleStart.value, visibleEnd.value)
    }
    
    const newWidths = { ...columnWidths.value }
    
    columns.forEach(column => {
      newWidths[column.key] = calculateColumnWidth(
        column.key,
        packetsToCheck,
        newWidths[column.key] || defaultColumnWidths[column.key]
      )
    })
    
    columnWidths.value = newWidths
    emit('update:columnWidths', columnWidths.value)
  }, 100)
}

// 虚拟滚动：计算可见区域 - 修复核心问题
const updateVisibleRange = () => {
  if (!tbodyRef.value) return
  
  const containerHeight = tbodyRef.value.clientHeight
  const totalRows = props.packets?.length || 0
  totalHeight.value = totalRows * ROW_HEIGHT
  
  // 计算可见区域
  const start = Math.floor(scrollTop.value / ROW_HEIGHT)
  const visibleCount = Math.ceil(containerHeight / ROW_HEIGHT)
  
  visibleStart.value = Math.max(0, start - VISIBLE_OVERSCAN)
  visibleEnd.value = Math.min(totalRows, start + visibleCount + VISIBLE_OVERSCAN)
}

// 处理滚动事件
const handleScroll = (e) => {
  scrollTop.value = e.target.scrollTop
  updateVisibleRange()
}

// 监听 packets 变化 - 优化：只更新可见区域，列宽计算有防抖
watch(() => props.packets, (newPackets) => {
  if (!newPackets) return
  
  const totalPackets = newPackets.length
  
  // 立即更新可见区域
  updateVisibleRange()
  
  // 优化：只在包数量较少或达到阈值时计算列宽
  if (totalPackets < 500 || totalPackets % 50 === 0) {
    calculateColumnWidths()
  }
}, { immediate: true })

// 监听 visibleStart/End 变化，更新可见数据包
watch([visibleStart, visibleEnd], () => {
  const totalPackets = props.packets?.length || 0
  if (totalPackets >= 1000) {
    // 大数据量时，可见区域变化后重新计算列宽
    if (calculateTimeout) clearTimeout(calculateTimeout)
    calculateTimeout = setTimeout(() => {
      calculateColumnWidths()
    }, 150)
  }
})

// 监听 columnWidths props 变化
watch(() => props.columnWidths, (newWidths) => {
  if (newWidths && JSON.stringify(newWidths) !== JSON.stringify(columnWidths.value)) {
    columnWidths.value = { ...newWidths }
  }
}, { deep: true })

// 检查某行是否在可视区域内
const isRowInViewport = (index) => {
  if (!tbodyRef.value) return false
  
  const rowTop = index * ROW_HEIGHT
  const rowBottom = rowTop + ROW_HEIGHT
  const viewportTop = scrollTop.value
  const viewportBottom = scrollTop.value + tbodyRef.value.clientHeight
  
  // 留 2 行边距
  const margin = ROW_HEIGHT * 2
  return rowTop >= viewportTop - margin && rowBottom <= viewportBottom + margin
}

// 暴露方法
const scrollTo = (index, options = {}) => {
  if (!tbodyRef.value || index < 0) return
  
  const top = index * ROW_HEIGHT
  const containerHeight = tbodyRef.value.clientHeight
  
  // 检查是否在可视区域内
  const isInViewport = isRowInViewport(index)
  
  if (options.smart !== false && isInViewport) {
    // 已经在可视区域，只更新选中状态，不滚动
    return
  }
  
  // 不在可视区域，滚动到居中位置
  const centeredTop = top - (containerHeight / 2) + (ROW_HEIGHT / 2)
  const scrollTarget = Math.max(0, centeredTop)
  
  tbodyRef.value.scrollTo({ 
    top: scrollTarget, 
    behavior: options.behavior || 'smooth' 
  })
  scrollTop.value = scrollTarget
  
  // 等待滚动完成后更新可见区域
  setTimeout(() => {
    updateVisibleRange()
  }, options.behavior === 'smooth' ? 300 : 50)
}

// 获取可见区域的数据包 - 使用 computed 确保响应式
const visiblePackets = computed(() => {
  if (!props.packets) return []
  const start = visibleStart.value
  const end = visibleEnd.value
  return props.packets.slice(start, end)
})

// 暴露 updateVisibleRange 供父组件调用
defineExpose({
  scrollTo,
  calculateColumnWidths,
  updateVisibleRange,
  isRowInViewport
})

// 拖拽调整
const startResize = (e, columnKey) => {
  isResizing.value = true
  resizingColumn.value = columnKey
  resizeStartX.value = e.clientX
  resizeStartWidth.value = columnWidths.value[columnKey] || defaultColumnWidths[columnKey]
  document.body.style.userSelect = 'none'
  document.body.style.cursor = 'col-resize'
}

const doResize = (e) => {
  if (!isResizing.value || !resizingColumn.value) return
  const diff = e.clientX - resizeStartX.value
  const newWidth = Math.max(minColumnWidths[resizingColumn.value] || 40, resizeStartWidth.value + diff)
  columnWidths.value[resizingColumn.value] = newWidth
  emit('update:columnWidths', columnWidths.value)
}

const stopResize = () => {
  // 拖动结束后，重新计算列宽确保对齐
  if (resizingColumn.value) {
    nextTick(() => {
      calculateColumnWidths(true)
    })
  }
  
  isResizing.value = false
  resizingColumn.value = null
  document.body.style.userSelect = ''
  document.body.style.cursor = ''
}

onMounted(() => {
  document.addEventListener('mousemove', doResize)
  document.addEventListener('mouseup', stopResize)
  calculateColumnWidths()
  
  // 确保 scrollToIndex 被正确暴露
  nextTick(() => {
    emit('scrollToIndex', scrollTo)
    updateVisibleRange()
  })
})

onUnmounted(() => {
  document.removeEventListener('mousemove', doResize)
  document.removeEventListener('mouseup', stopResize)
  if (calculateTimeout) clearTimeout(calculateTimeout)
})

// PacketRow 子组件逻辑
const getPacketLength = (packet) => {
  return packet?.packet?.length || 0
}

const getPacketString = (packet) => {
  const obj = packet?.object || {}
  return JSON.stringify(obj).substring(0, 200)
}

const getColumnWidth = (key, widths) => {
  const width = widths?.[key] ?? defaultColumnWidths[key]
  return width === 'auto' ? 'auto' : width + 'px'
}
</script>

<template>
  <div class="table">
    <!-- 表头 -->
    <div ref="headerRef" class="tr thead">
      <template v-for="(column, index) in columns" :key="column.key">
        <div
          class="col"
          :class="column.key"
          :style="{
            flex: `0 0 ${columnWidths[column.key] === 'auto' ? '1fr' : columnWidths[column.key] + 'px'}`,
            width: columnWidths[column.key] === 'auto' ? 'auto' : columnWidths[column.key] + 'px'
          }"
        >
          {{ column.label }}
        </div>
        <div
          v-if="index < columns.length - 1"
          class="resize-handle"
          @mousedown="startResize($event, column.key)"
        ></div>
      </template>
    </div>

    <!-- 表体 - 虚拟滚动 -->
    <div 
      ref="tbodyRef" 
      class="tbody"
      @scroll="handleScroll"
    >
      <!-- 占位元素，维持总高度 -->
      <div class="virtual-spacer" :style="{ height: totalHeight + 'px' }">
        <!-- 只渲染可见区域的行 -->
        <div
          v-for="(packet, localIndex) in visiblePackets"
          :key="packet.globalId || (visibleStart + localIndex)"
          class="tr"
          :class="{ 
            current: packet === currentPacket, 
            o: (visibleStart + localIndex) % 2 === 1 
          }"
          :style="{
            transform: `translateY(${(visibleStart + localIndex) * ROW_HEIGHT}px)`,
            position: 'absolute',
            top: 0,
            left: 0,
            right: 0,
            height: ROW_HEIGHT + 'px'
          }"
          @click="$emit('selectPacket', packet)"
        >
          <div
            class="col time"
            :style="{
              flex: `0 0 ${getColumnWidth('time', columnWidths)}`,
              width: getColumnWidth('time', columnWidths)
            }"
          >
            {{ getCellValue(packet, 'time') }}
          </div>
          <div
            class="col idx"
            :style="{
              flex: `0 0 ${getColumnWidth('idx', columnWidths)}`,
              width: getColumnWidth('idx', columnWidths)
            }"
          >
            {{ packet.index }}
          </div>
          <div
            class="col src"
            :style="{
              flex: `0 0 ${getColumnWidth('src', columnWidths)}`,
              width: getColumnWidth('src', columnWidths)
            }"
          >
            <span :class="packet.source ? 'client' : 'server'">
              {{ packet.source ? 'CLIENT' : 'SERVER' }}
            </span>
          </div>
          <div
            class="col id"
            :style="{
              flex: `0 0 ${getColumnWidth('id', columnWidths)}`,
              width: getColumnWidth('id', columnWidths)
            }"
          >
            {{ packet.packetID }}
          </div>
          <div
            class="col name"
            :style="{
              flex: `0 0 ${getColumnWidth('name', columnWidths)}`,
              width: getColumnWidth('name', columnWidths)
            }"
          >
            {{ packet.protoName }}
          </div>
          <div
            class="col len"
            :style="{
              flex: `0 0 ${getColumnWidth('len', columnWidths)}`,
              width: getColumnWidth('len', columnWidths)
            }"
          >
            {{ getPacketLength(packet) || '' }}
          </div>
          <div
            class="col json"
            :style="{
              flex: getColumnWidth('json', columnWidths) === 'auto' ? '1 1 0' : `0 0 ${getColumnWidth('json', columnWidths)}`,
              width: getColumnWidth('json', columnWidths)
            }"
          >
            <span>{{ getPacketString(packet) }}</span>
          </div>
        </div>
      </div>
      
      <div v-if="packets.length === 0" class="empty-state">
        <p>No packets captured</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 表格容器样式 */
.table {
  width: 100%;
  min-width: 100%;
  cursor: default;
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  overflow-x: hidden;
}

.tr {
  display: flex;
  width: 100%;
  cursor: pointer;
  flex-shrink: 0;
  align-items: center;
  transition: background 0.15s;
  border-bottom: 1px solid var(--border-light);
  overflow: visible;
  will-change: transform;
  contain: layout style;
}

.tr:hover {
  background: var(--hover-bg) !important;
}

.tr.o {
  background: var(--bg-secondary);
}

.tr.current {
  background: var(--selected-bg) !important;
}

.tr.current > .col {
  background: var(--selected-bg);
  color: var(--selected-text);
}

.tr.current .col.src span {
  border-color: var(--text-light);
}

/* 表头样式 */
.thead {
  flex-shrink: 0;
  position: relative;
  background: var(--bg-tertiary);
  border-bottom: 2px solid var(--border-medium);
  z-index: 5;
  will-change: auto;
}

.thead .col {
  text-align: left;
  background: var(--bg-tertiary);
  border-right: 1px solid var(--border-medium);
  font-size: 0.7em;
  font-weight: 700;
  padding: 0.75rem 0.6rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  display: flex;
  align-items: center;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.8px;
  box-sizing: border-box;
  transition: background 0.15s;
}

.thead .col:last-child {
  border-right: none;
}

.thead .col:hover {
  background: var(--hover-bg);
}

/* 表体样式 */
.tbody {
  flex-grow: 1;
  flex-shrink: 1;
  overflow-y: auto;
  overflow-x: hidden;
  position: relative;
  contain: strict;
  will-change: scroll-position;
}

/* 虚拟占位元素 */
.virtual-spacer {
  position: relative;
  width: 100%;
  will-change: height;
}

/* 列通用样式 */
.col {
  padding: 0.6rem 0.6rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  flex-shrink: 0;
  border-right: 1px solid var(--border-light);
  font-size: 0.85em;
  color: var(--text-primary);
  box-sizing: border-box;
  will-change: auto;
}

.col:last-child {
  border-right: none;
}

.col.time { 
  justify-content: center; 
  font-family: 'JetBrains Mono', monospace;
  white-space: nowrap;
  overflow: hidden;
}

.col.idx { 
  justify-content: center; 
  font-family: 'JetBrains Mono', monospace;
}

.col.src { 
  justify-content: center; 
}

.col.id { 
  justify-content: center; 
  font-family: 'JetBrains Mono', monospace; 
  font-weight: 600;
}

.col.len { 
  font-family: 'JetBrains Mono', monospace; 
  font-weight: 600;
}

.col.json { 
  flex-grow: 1;
  overflow: visible;
}

/* 发送方徽章样式 */
.col.src span {
  border: 1px solid var(--border-medium);
  display: inline-block;
  border-radius: 4px;
  padding: 0.15em 0.4em;
  font-size: 0.75em;
  font-weight: 600;
  text-transform: uppercase;
}

.col.src span.client {
  background: var(--client-bg);
  color: var(--client-text);
  border-color: var(--client-bg);
}

.col.src span.server {
  background: var(--server-bg);
  color: var(--server-text);
  border-color: var(--server-bg);
}

/* JSON 预览样式 */
.col.json span {
  display: block;
  white-space: nowrap;
  overflow: visible;
  text-overflow: clip;
  line-height: 1.4;
  color: var(--text-secondary);
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.85em;
}

/* 拖拽手柄样式 */
.resize-handle {
  width: 4px;
  height: 100%;
  cursor: col-resize;
  background: transparent;
  flex-shrink: 0;
  z-index: 10;
  transition: background 0.2s;
  margin: 0 -2px;
}

.resize-handle:hover {
  background: var(--primary-color);
}

/* 空状态样式 */
.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-tertiary);
  font-size: 1.1em;
}
</style>
