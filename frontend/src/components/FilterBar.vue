<script setup>
import { ref } from 'vue'

const emit = defineEmits(['update:filter', 'update:jsonFilter', 'update:useAnd'])

const filterText = ref('')
const jsonFilter = ref('')
const useAnd = ref(true)
</script>

<template>
  <div class="filter-host">
    <input
      type="text"
      v-model="filterText"
      @input="emit('update:filter', filterText)"
      placeholder="🔍 PACKET"
    />
    <div class="orand" @click="useAnd = !useAnd; emit('update:useAnd', useAnd)">
      <span :class="{ s: useAnd }" and>AND</span>
      <span :class="{ s: !useAnd }" or>OR</span>
    </div>
    <input
      type="text"
      v-model="jsonFilter"
      @input="emit('update:jsonFilter', jsonFilter)"
      placeholder="🔍 JSON"
    />
  </div>
</template>

<style scoped>
.filter-host {
  width: 100%;
  display: flex;
  background: var(--bg-primary);
  flex-shrink: 0;
  border-bottom: 1px solid var(--border-light);
}

.filter-host input[type="text"] {
  background: var(--bg-primary);
  color: var(--text-primary);
  height: 3rem;
  border: none;
  border-right: 1px solid var(--border-light);
  padding: 0 1rem;
  flex-grow: 1;
  font-size: 1em;
  font-family: 'JetBrains Mono', monospace;
  border-radius: 0;
}

.filter-host input[type="text"]:focus {
  outline: none;
  background: var(--bg-tertiary);
}

.filter-host input[type="text"]::placeholder {
  font-family: inherit;
  color: var(--text-tertiary);
  font-size: 0.9em;
}

.orand {
  font-size: 0.75em;
  box-sizing: border-box;
  padding: 4px 2px;
  background: var(--bg-tertiary);
  border-right: 1px solid var(--border-light);
  cursor: pointer;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  width: 42px;
  height: 3rem;
  font-weight: 600;
  gap: 2px;
}

.orand:hover {
  background: var(--bg-secondary);
}

.orand span {
  color: var(--text-tertiary);
  font-weight: 700;
  padding: 3px 6px;
  display: block;
  border-radius: 4px;
  min-width: 28px;
  text-align: center;
  user-select: none;
  line-height: 1.2;
  transition: all 0.15s;
  width: 100%;
}

.orand span[and] {
  margin-bottom: 0;
}

.orand span.s {
  background: var(--primary-color);
  color: var(--text-light);
  box-shadow: 0 1px 2px rgba(37, 99, 235, 0.3);
}
</style>
