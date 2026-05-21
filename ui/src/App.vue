<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import Sidebar from './components/Sidebar.vue'
import Detail from './components/Detail.vue'
import Settings from './components/Settings.vue'
import Welcome from './components/Welcome.vue'

interface InstanceInfo {
  name: string
  minecraft_version: string
  loader_type: string | null
  loader_version: string | null
}

interface ConfigInfo {
  data_dir: string
  max_concurrent_downloads: number
  accounts: string[]
  active_account: number | null
}

const view = ref<'welcome' | 'main' | 'settings'>('main')
const instances = ref<InstanceInfo[]>([])
const selected = ref<string | null>(null)
const status = ref('Ready')
const config = ref<ConfigInfo | null>(null)

async function loadInstances() {
  instances.value = await invoke('list_instances')
}

async function loadConfig() {
  config.value = await invoke('get_config')
  if (!config.value || (config.value.accounts.length === 0 && instances.value.length === 0)) {
    view.value = 'welcome'
  }
}

onMounted(async () => {
  await loadInstances()
  await loadConfig()
})

function selectInstance(name: string) {
  selected.value = name
}
</script>

<template>
  <div id="app">
    <Welcome v-if="view === 'welcome'" @done="view = 'main'; loadInstances(); loadConfig()" />

    <template v-else-if="view === 'settings'">
      <header class="flex items-center px-4 py-3 bg-[#16181e] border-b border-[#2a2d36]">
        <button @click="view = 'main'" class="text-sm text-[#78b4ff] hover:text-white transition">← Back</button>
        <h1 class="ml-4 text-lg font-semibold text-[#78b4ff]">Settings</h1>
      </header>
      <Settings :config="config" @update="loadConfig(); loadInstances()" />
    </template>

    <template v-else>
      <header class="flex items-center justify-between px-4 py-3 bg-[#16181e] border-b border-[#2a2d36]">
        <div class="flex items-center gap-3">
          <h1 class="text-lg font-bold text-[#78b4ff]">MiaoMC</h1>
          <span class="text-xs text-[#6b7080]">Minecraft Launcher</span>
        </div>
        <button @click="view = 'settings'" class="px-3 py-1.5 text-sm rounded-md bg-[#2d323c] hover:bg-[#3c4250] transition">Settings</button>
      </header>
      <div class="flex flex-1 overflow-hidden">
        <Sidebar :instances="instances" :selected="selected" @select="selectInstance" @refresh="loadInstances" />
        <Detail :instance-name="selected" :status="status" @status="(s: string) => status = s" @refresh="loadInstances" />
      </div>
      <footer class="px-4 py-2 text-xs text-[#6b7080] bg-[#16181e] border-t border-[#2a2d36]">{{ status }}</footer>
    </template>
  </div>
</template>
