<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface ConfigInfo {
  data_dir: string
  max_concurrent_downloads: number
  accounts: string[]
  active_account: number | null
}

const props = defineProps<{ config: ConfigInfo | null }>()
const emit = defineEmits<{ update: [] }>()

const dataDir = ref('')
const maxDownloads = ref(8)
const username = ref('')
const javas = ref<any[]>([])

onMounted(async () => {
  if (props.config) {
    dataDir.value = props.config.data_dir
    maxDownloads.value = props.config.max_concurrent_downloads
  }
  javas.value = await invoke('detect_java')
})

async function saveGeneral() {
  await invoke('save_config', { dataDir: dataDir.value, maxDownloads: maxDownloads.value })
  emit('update')
}

async function addOffline() {
  if (!username.value) return
  await invoke('add_offline_account', { username: username.value })
  username.value = ''
  emit('update')
}

async function refreshJava() {
  javas.value = await invoke('detect_java')
}
</script>

<template>
  <div class="flex-1 overflow-y-auto p-6">
    <div class="max-w-xl mx-auto space-y-6">
      <!-- General -->
      <section class="p-5 rounded-2xl bg-[#262a34] border border-[#2a2d36]">
        <h2 class="text-sm font-semibold text-[#a0a5b4] mb-4">General</h2>
        <label class="text-xs text-[#6b7080] block mb-1">Data Directory</label>
        <div class="flex gap-2 mb-3">
          <input v-model="dataDir" class="flex-1 px-3 py-2 text-sm rounded-lg bg-[#1a1d24] border border-[#2a2d36] text-white focus:border-[#4b82c3] focus:outline-none transition" />
        </div>
        <label class="text-xs text-[#6b7080] block mb-1">Max Concurrent Downloads</label>
        <input v-model.number="maxDownloads" type="number" min="1" max="32" class="w-20 px-3 py-2 text-sm rounded-lg bg-[#1a1d24] border border-[#2a2d36] text-white focus:border-[#4b82c3] focus:outline-none transition mb-4" />
        <div>
          <button @click="saveGeneral" class="px-4 py-2 text-sm rounded-lg bg-[#4b82c3] hover:bg-[#5a92d3] text-white transition">Save</button>
        </div>
      </section>

      <!-- Java -->
      <section class="p-5 rounded-2xl bg-[#262a34] border border-[#2a2d36]">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-sm font-semibold text-[#a0a5b4]">Java Installations</h2>
          <button @click="refreshJava" class="text-xs text-[#78b4ff] hover:text-white transition">Refresh</button>
        </div>
        <div v-if="javas.length === 0" class="text-xs text-[#6b7080]">None detected</div>
        <div v-else class="space-y-2">
          <div v-for="j in javas" :key="j.path" class="text-sm text-[#c8cdd7]">
            Java {{ j.major_version }} ({{ j.version }}) — <span class="text-[#6b7080]">{{ j.path }}</span>
          </div>
        </div>
      </section>

      <!-- Accounts -->
      <section class="p-5 rounded-2xl bg-[#262a34] border border-[#2a2d36]">
        <h2 class="text-sm font-semibold text-[#a0a5b4] mb-4">Accounts</h2>
        <div v-if="!config?.accounts?.length" class="text-xs text-[#6b7080] mb-3">No accounts configured</div>
        <div v-else class="space-y-1 mb-4">
          <div v-for="(acc, i) in config!.accounts" :key="i" class="flex items-center gap-2 text-sm text-[#c8cdd7]">
            <span :class="config!.active_account === i ? 'text-[#50b450]' : 'text-[#3c3c3c]'">●</span>
            {{ acc }}
          </div>
        </div>
        <div class="flex gap-2">
          <input v-model="username" placeholder="Username" class="flex-1 px-3 py-2 text-sm rounded-lg bg-[#1a1d24] border border-[#2a2d36] text-white placeholder-[#6b7080] focus:border-[#4b82c3] focus:outline-none transition" />
          <button @click="addOffline" class="px-4 py-2 text-sm rounded-lg bg-[#2d323c] hover:bg-[#3c4250] text-white transition">Add Offline</button>
        </div>
      </section>
    </div>
  </div>
</template>
