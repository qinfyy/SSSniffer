<script setup>
import { ref } from 'vue'

const props = defineProps({
  sessionStarted: Boolean,
  currentPacket: Object,
  hasFilteredPackets: Boolean,
  stick: Boolean,
  isDarkTheme: Boolean
})

const emit = defineEmits(['toggleSession', 'clear', 'openSettings', 'toggleStick', 'toggleTheme', 'upload', 'copyBin', 'copyPacket'])

const fileInput = ref(null)

const triggerUpload = () => {
  fileInput.value?.click()
}

const handleFileChange = (e) => {
  const file = e.target.files?.[0]
  if (file) {
    emit('upload', file)
    e.target.value = ''
  }
}
</script>

<template>
  <aside class="sidebar">
    <button
      :class="['session-btn', sessionStarted ? 'stop' : 'start']"
      :title="sessionStarted ? 'Stop Capture' : 'Start Capture'"
      @click="emit('toggleSession')"
    >
      {{ sessionStarted ? '⏹' : '▶' }}
    </button>

    <button title="Upload PCAP" @click="triggerUpload">📁</button>
    <input
      ref="fileInput"
      type="file"
      hidden
      accept=".json,.pcap,.pcapng,.gcap"
      @change="handleFileChange"
    />
    <button title="Clear" class="danger" @click="emit('clear')">🗑</button>
    <button title="Settings" @click="emit('openSettings')">⚙</button>

    <div class="sidebar-bottom">
      <!-- 主题切换按钮 -->
      <button
        :class="['theme-btn', { active: isDarkTheme }]"
        :title="isDarkTheme ? 'Switch to Light Theme' : 'Switch to Dark Theme'"
        @click="emit('toggleTheme')"
      >
        {{ isDarkTheme ? '☀️' : '🌙' }}
      </button>

      <button
        :class="['stick-btn', { active: stick }]"
        title="Lock scroll at the bottom"
        @click="emit('toggleStick')"
      >
        ⬇
      </button>

      <button
        :disabled="!currentPacket"
        title="Copy current bin"
        @click="emit('copyBin')"
      >
        📄
      </button>

      <button
        :disabled="!currentPacket"
        title="Copy current packet"
        @click="emit('copyPacket')"
      >
        📋
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  background: var(--bg-dark);
  flex-grow: 0;
  display: flex;
  flex-direction: column;
  padding: 0.75rem 0.25rem;
  gap: 0.25rem;
  flex-shrink: 0;
  width: 48px;
  transition: background 0.3s ease;
}

.sidebar button {
  font-size: 1.3em;
  background: transparent;
  border: none;
  width: 40px;
  height: 40px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  color: var(--text-light);
  transition: all 0.15s ease;
}

.sidebar button:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.1);
}

.session-btn.start {
  color: var(--success);
}

.session-btn.stop {
  color: var(--danger);
}

.danger {
  color: var(--danger);
}

.sidebar-bottom {
  margin-top: auto;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.stick-btn.active {
  color: var(--primary-color);
  background: rgba(37, 99, 235, 0.2);
}

/* 主题切换按钮样式 */
.theme-btn {
  color: var(--text-light);
}

.theme-btn:hover {
  background: rgba(255, 255, 255, 0.15);
}

.theme-btn.active {
  color: #fbbf24;
  background: rgba(251, 191, 36, 0.2);
}
</style>
