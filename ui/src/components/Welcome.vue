<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const emit = defineEmits<{ done: [] }>()
const dataDir = ref('')

async function init() {
  const config: any = await invoke('get_config')
  dataDir.value = config.data_dir
}
init()

async function start() {
  if (dataDir.value) {
    await invoke('save_config', { dataDir: dataDir.value, maxDownloads: 8 })
  }
  emit('done')
}
</script>

<template>
  <div class="flex-1 flex items-center justify-center">
    <div class="text-center max-w-md">
      <h1 class="text-3xl font-bold text-[#78b4ff] mb-2">MiaoMC</h1>
      <p class="text-sm text-[#a0a5b4] mb-10">A lightweight Minecraft launcher for Linux</p>

      <div class="p-6 rounded-2xl bg-[#262a34] border border-[#2a2d36] text-left">
        <label class="text-sm font-medium text-[#a0a5b4] block mb-2">Data Directory</label>
        <p class="text-xs text-[#6b7080] mb-3">Where instances, libraries, and assets are stored</p>
        <input
          v-model="dataDir"
          class="w-full px-3 py-2 text-sm rounded-lg bg-[#1a1d24] border border-[#2a2d36] text-white placeholder-[#6b7080] focus:border-[#4b82c3] focus:outline-none transition"
        />
      </div>

      <button
        @click="start"
        class="mt-8 px-8 py-3 text-sm font-semibold rounded-xl bg-[#4b82c3] hover:bg-[#5a92d3] text-white transition shadow-lg shadow-blue-900/30"
      >
        Get Started
      </button>
    </div>
  </div>
</template>
