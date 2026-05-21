<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'

interface InstanceInfo {
  name: string
  minecraft_version: string
  loader_type: string | null
  loader_version: string | null
}

defineProps<{
  instances: InstanceInfo[]
  selected: string | null
}>()

const emit = defineEmits<{
  select: [name: string]
  refresh: []
}>()

async function importPack() {
  const { open } = await import('@tauri-apps/plugin-dialog')
  const file = await open({ filters: [{ name: 'Modpack', extensions: ['mrpack'] }] })
  if (file) {
    await invoke('import_mrpack', { path: file })
    emit('refresh')
  }
}
</script>

<template>
  <aside class="w-60 flex flex-col bg-[#1e2128] border-r border-[#2a2d36]">
    <div class="flex items-center justify-between px-4 py-3">
      <span class="text-sm font-semibold text-[#a0a5b4]">Instances</span>
      <div class="flex gap-1">
        <button @click="$emit('refresh')" class="px-2 py-1 text-xs rounded bg-[#2d323c] hover:bg-[#3c4250] transition">+ New</button>
        <button @click="importPack" class="px-2 py-1 text-xs rounded bg-[#2d323c] hover:bg-[#3c4250] transition">Import</button>
      </div>
    </div>
    <div class="flex-1 overflow-y-auto px-2 pb-2">
      <div v-if="instances.length === 0" class="text-center text-sm text-[#6b7080] mt-8">
        <p>No instances yet</p>
        <p class="text-xs mt-2">Click '+ New' to create one</p>
      </div>
      <button
        v-for="inst in instances"
        :key="inst.name"
        @click="$emit('select', inst.name)"
        :class="[
          'w-full text-left px-3 py-2.5 rounded-lg mb-1 transition-all',
          selected === inst.name
            ? 'bg-[#4b82c3]/20 text-white border border-[#4b82c3]/40'
            : 'hover:bg-[#2d323c] text-[#c8cdd7]'
        ]"
      >
        <div class="text-sm font-medium">{{ inst.name }}</div>
        <div class="text-xs text-[#6b7080] mt-0.5">
          MC {{ inst.minecraft_version }}
          <span v-if="inst.loader_type" class="ml-1 text-[#50b450]">{{ inst.loader_type }}</span>
        </div>
      </button>
    </div>
  </aside>
</template>
