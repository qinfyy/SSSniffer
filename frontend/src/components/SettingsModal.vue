<script setup>
import { ref, watch } from 'vue'

const props = defineProps({
  settings: Object
})

const emit = defineEmits(['close', 'save'])

const startPort = ref('')
const endPort = ref('')
const useTcp = ref(false)
const autoSavePcapFiles = ref(true)
const autoSaveTextFiles = ref(true)
const packetFilterList = ref([])
const newProtocol = ref('')

const editingIndex = ref(-1)
const editingValue = ref('')

watch(() => props.settings, (newSettings) => {
  if (newSettings) {
    startPort.value = newSettings.startPort || ''
    endPort.value = newSettings.endPort || ''
    useTcp.value = newSettings.useTcp || false
    autoSavePcapFiles.value = newSettings.autoSavePcapFiles ?? true
    autoSaveTextFiles.value = newSettings.autoSaveTextFiles ?? true
    packetFilterList.value = [...(newSettings.packetFilterList || [])]
  }
}, { immediate: true })

const addProtocol = () => {
  const protocol = newProtocol.value.trim()
  if (protocol && !packetFilterList.value.includes(protocol)) {
    packetFilterList.value.push(protocol)
    newProtocol.value = ''
  }
}

const startEdit = (index) => {
  editingIndex.value = index
  editingValue.value = packetFilterList.value[index]
}

const saveEdit = () => {
  if (editingIndex.value >= 0) {
    const value = editingValue.value.trim()
    if (value) {
      packetFilterList.value = packetFilterList.value.map((p, i) =>
        i === editingIndex.value ? value : p
      )
    } else {
      packetFilterList.value = packetFilterList.value.filter((_, i) => i !== editingIndex.value)
    }
    editingIndex.value = -1
    editingValue.value = ''
  }
}

const cancelEdit = () => {
  editingIndex.value = -1
  editingValue.value = ''
}

const removeProtocol = (index) => {
  packetFilterList.value = packetFilterList.value.filter((_, i) => i !== index)
  if (editingIndex.value === index) {
    editingIndex.value = -1
    editingValue.value = ''
  }
}

const handleSave = () => {
  if (!startPort.value || !endPort.value) {
    alert('Please fill in the complete port range')
    return
  }
  
  if (parseInt(startPort.value) > parseInt(endPort.value)) {
    alert('The start port cannot be larger than the end port')
    return
  }
  
  const newSettings = {
    startPort: startPort.value,
    endPort: endPort.value,
    useTcp: useTcp.value,
    autoSavePcapFiles: autoSavePcapFiles.value,
    autoSaveTextFiles: autoSaveTextFiles.value,
    packetFilterList: packetFilterList.value.filter(p => p.trim() !== '')
  }
  
  emit('save', newSettings)
}
</script>

<template>
  <div class="modal-overlay" @click="$emit('close')">
    <div class="settings-dialog compact" @click.stop>
      <h2>Capture Settings</h2>

      <div class="setting-row">
        <label>Port Range:</label>
        <div class="port-inputs">
          <input
            v-model="startPort"
            type="number"
            min="1"
            max="65535"
            placeholder="Start"
            class="small-input"
          />
          <span class="dash">–</span>
          <input
            v-model="endPort"
            type="number"
            min="1"
            max="65535"
            placeholder="End"
            class="small-input"
          />
        </div>
      </div>

      <div class="setting-row">
        <label>Use TCP:</label>
        <label class="checkbox-label">
          <input v-model="useTcp" type="checkbox" class="custom-checkbox" />
          <span class="checkmark"></span>
          {{ useTcp ? 'On' : 'Off' }}
        </label>
      </div>

      <div class="setting-row">
        <label>Save PCAP:</label>
        <label class="checkbox-label">
          <input v-model="autoSavePcapFiles" type="checkbox" class="custom-checkbox" />
          <span class="checkmark"></span>
          {{ autoSavePcapFiles ? 'On' : 'Off' }}
        </label>
      </div>

      <div class="setting-row">
        <label>Save Text:</label>
        <label class="checkbox-label">
          <input v-model="autoSaveTextFiles" type="checkbox" class="custom-checkbox" />
          <span class="checkmark"></span>
          {{ autoSaveTextFiles ? 'On' : 'Off' }}
        </label>
      </div>

      <div class="setting-group compact">
        <h3>Packet Filter</h3>
        <div class="protocol-table compact">
          <div
            v-for="(protocol, index) in packetFilterList"
            :key="index"
            class="filter-item"
          >
            <template v-if="editingIndex === index">
              <input
                v-model="editingValue"
                type="text"
                @keydown="(e) => {
                  if (e.key === 'Enter') saveEdit();
                  if (e.key === 'Escape') cancelEdit();
                }"
                autofocus
                class="filter-input editing"
              />
              <div class="filter-actions">
                <button @click="saveEdit" title="Save" class="btn-icon save">✓</button>
                <button @click="cancelEdit" title="Cancel" class="btn-icon cancel">✗</button>
              </div>
            </template>
            <template v-else>
              <span class="filter-text" @dblclick="startEdit(index)">{{ protocol }}</span>
              <div class="filter-actions">
                <button @click="startEdit(index)" title="Edit" class="btn-icon edit">✎</button>
                <button @click="removeProtocol(index)" title="Remove" class="btn-icon remove">×</button>
              </div>
            </template>
          </div>
          
          <div class="filter-item add">
            <input
              v-model="newProtocol"
              type="text"
              placeholder="Enter new protocol name"
              @keydown="(e) => e.key === 'Enter' && addProtocol()"
              class="filter-input"
            />
            <button @click="addProtocol" title="Add" class="btn-icon add-btn">+</button>
          </div>
        </div>
      </div>

      <div class="settings-actions compact">
        <button class="btn secondary" @click="$emit('close')">
          Cancel
        </button>
        <button class="btn primary" @click="handleSave">
          Save
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 深色主题 CSS */
:root {
  --bg-primary: #1e1e2e;
  --bg-secondary: #252538;
  --bg-tertiary: #2d2d44;
  --border-light: #3a3a55;
  --border-medium: #4a4a6a;
  --border-focus: #6366f1;
  --border-focus-glow: rgba(99, 102, 241, 0.3);
  --text-primary: #e4e4e7;
  --text-secondary: #a1a1aa;
  --text-tertiary: #71717a;
  --primary-color: #6366f1;
  --primary-hover: #4f46e5;
  --primary-light: rgba(99, 102, 241, 0.2);
  --success: #10b981;
  --success-hover: #059669;
  --danger: #ef4444;
  --danger-hover: #dc2626;
  --info: #3b82f6;
  --info-hover: #2563eb;
}

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 1000;
  backdrop-filter: blur(4px);
}

.settings-dialog.compact {
  background: var(--bg-primary);
  border-radius: 12px;
  padding: 1.5rem;
  min-width: 450px;
  max-width: 90%;
  max-height: 80vh;
  overflow: auto;
  border: 1px solid var(--border-light);
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  font-size: 0.9em;
}

.settings-dialog.compact h2 {
  margin: 0 0 1.5rem 0;
  font-size: 1.3rem;
  color: var(--text-primary);
  text-align: center;
  font-weight: 600;
}

.setting-row {
  display: flex;
  align-items: center;
  margin-bottom: 1rem;
  gap: 1rem;
}

.setting-row label:first-child {
  width: 100px;
  color: var(--text-secondary);
  font-weight: 500;
}

.port-inputs {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex: 1;
}

.small-input {
  padding: 0.5rem 0.75rem;
  background: var(--bg-secondary);
  border: 1px solid var(--border-medium);
  color: var(--text-primary);
  border-radius: 6px;
  font-size: 0.9em;
  width: 80px;
  transition: all 0.2s ease;
}

.small-input::placeholder {
  color: var(--text-tertiary);
}

.small-input:focus {
  border-color: var(--border-focus);
  box-shadow: 0 0 0 3px var(--border-focus-glow);
  outline: none;
  background: var(--bg-tertiary);
}

.dash {
  color: var(--text-tertiary);
  font-weight: 600;
}

/* 复选框样式 */
.checkbox-label {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  cursor: pointer;
  color: var(--text-primary);
  position: relative;
}

.checkbox-label input[type="checkbox"] {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
}

.checkmark {
  width: 18px;
  height: 18px;
  border: 2px solid var(--border-medium);
  border-radius: 4px;
  background: var(--bg-secondary);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
  flex-shrink: 0;
}

.checkbox-label:hover .checkmark {
  border-color: var(--border-focus);
  background: var(--bg-tertiary);
}

.checkbox-label input[type="checkbox"]:checked + .checkmark {
  background: var(--primary-color);
  border-color: var(--primary-color);
}

.checkbox-label input[type="checkbox"]:checked + .checkmark::after {
  content: '✓';
  color: white;
  font-size: 12px;
  font-weight: bold;
}

.checkbox-label input[type="checkbox"]:focus + .checkmark {
  box-shadow: 0 0 0 3px var(--border-focus-glow);
}

.setting-group.compact {
  margin: 1.5rem 0;
}

.setting-group.compact h3 {
  margin: 0 0 0.75rem 0;
  font-size: 0.9em;
  color: var(--text-secondary);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.protocol-table.compact {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 0.5rem;
  max-height: 150px;
  overflow-y: auto;
  border: 1px solid var(--border-light);
}

.protocol-table.compact::-webkit-scrollbar {
  width: 6px;
}

.protocol-table.compact::-webkit-scrollbar-track {
  background: var(--bg-primary);
  border-radius: 3px;
}

.protocol-table.compact::-webkit-scrollbar-thumb {
  background: var(--border-medium);
  border-radius: 3px;
}

.protocol-table.compact::-webkit-scrollbar-thumb:hover {
  background: var(--border-focus);
}

.filter-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.5rem;
  padding: 0.5rem;
  border-radius: 6px;
  background: var(--bg-primary);
  border: 1px solid transparent;
  transition: all 0.2s ease;
}

.filter-item:last-child {
  margin-bottom: 0;
}

.filter-item:hover {
  background: var(--bg-tertiary);
  border-color: var(--border-light);
}

.filter-item.add {
  background: var(--bg-tertiary);
  border: 1px dashed var(--border-medium);
}

.filter-input {
  flex: 1;
  padding: 0.4rem 0.6rem;
  background: var(--bg-primary);
  border: 1px solid var(--border-medium);
  color: var(--text-primary);
  border-radius: 4px;
  font-size: 0.85em;
  height: 32px;
  line-height: 1.2;
  transition: all 0.2s ease;
}

.filter-input::placeholder {
  color: var(--text-tertiary);
}

.filter-input:focus {
  border-color: var(--border-focus);
  box-shadow: 0 0 0 2px var(--border-focus-glow);
  outline: none;
  background: var(--bg-tertiary);
}

.filter-input.editing {
  border-color: var(--border-focus);
  background: var(--bg-tertiary);
}

.filter-text {
  flex: 1;
  color: var(--text-primary);
  font-size: 0.85em;
  padding: 0.25rem 0;
  cursor: pointer;
}

.filter-text:hover {
  color: var(--text-secondary);
}

.filter-actions {
  display: flex;
  gap: 0.25rem;
  flex-shrink: 0;
}

.btn-icon {
  width: 26px;
  height: 26px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  border: none;
  background: var(--bg-tertiary);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 0.8em;
  padding: 0;
  transition: all 0.2s ease;
}

.btn-icon:hover {
  transform: scale(1.05);
}

.btn-icon.save {
  background: var(--success);
  color: white;
}

.btn-icon.save:hover {
  background: var(--success-hover);
}

.btn-icon.cancel {
  background: var(--danger);
  color: white;
}

.btn-icon.cancel:hover {
  background: var(--danger-hover);
}

.btn-icon.edit {
  background: var(--info);
  color: white;
}

.btn-icon.edit:hover {
  background: var(--info-hover);
}

.btn-icon.remove {
  background: var(--danger);
  color: white;
}

.btn-icon.remove:hover {
  background: var(--danger-hover);
}

.add-btn {
  background: var(--primary-color);
  color: white;
}

.add-btn:hover {
  background: var(--primary-hover);
}

.settings-actions.compact {
  display: flex;
  justify-content: space-between;
  margin-top: 1.5rem;
  padding-top: 1rem;
  border-top: 1px solid var(--border-light);
}

.btn {
  padding: 0.6rem 1.5rem;
  border-radius: 6px;
  cursor: pointer;
  font-weight: 500;
  border: 1px solid var(--border-medium);
  font-size: 0.9em;
  transition: all 0.2s ease;
}

.btn.secondary {
  background: var(--bg-tertiary);
  color: var(--text-secondary);
}

.btn.secondary:hover {
  background: var(--border-medium);
  color: var(--text-primary);
}

.btn.primary {
  background: var(--primary-color);
  color: white;
  border-color: var(--primary-color);
}

.btn.primary:hover {
  background: var(--primary-hover);
  box-shadow: 0 0 15px var(--border-focus-glow);
}

.btn:focus {
  outline: none;
  box-shadow: 0 0 0 3px var(--border-focus-glow);
}
</style>
