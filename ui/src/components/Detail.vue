<script setup lang="ts">
import { ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import ModsTab from './tabs/ModsTab.vue'
import ResourcesTab from './tabs/ResourcesTab.vue'
import WorldsTab from './tabs/WorldsTab.vue'
import LogTab from './tabs/LogTab.vue'

const props = defineProps<{
  instanceName: string | null
  status: string
}>()

const emit = defineEmits<{
  status: [msg: string]
  refresh: []
}>()

const activeTab = ref<'mods' | 'resources' | 'worlds' | 'log'>('mods')
const instance = ref<any>(null)

watch(() => props.instanceName, async (name) => {
  if (!name) { instance.value = null; return }
  const instances: any[] = await invoke('list_instances')
  instance.value = instances.find(i => i.name === name) || null
}, { immediate: true })

async function launch() {
  if (!props.instanceName) return
  emit('status', `Launching ${props.instanceName}...`)
  try {
    const msg = await invoke('launch_instance', { name: props.instanceName }) as string
    emit('status', msg)
  } catch (e: any) {
    if (e.includes && e.includes('Java')) {
      emit('status', 'Downloading Java...')
      try {
        const major = parseInt(e.match(/Java (\d+)/)?.[1] || '21')
        await invoke('download_java', { majorVersion: major })
        emit('status', 'Java installed. Try launching again.')
      } catch (je: any) {
        emit('status', `Java download failed: ${je}`)
      }
    } else {
      emit('status', `Error: ${e}`)
    }
  }
}

async function deleteInstance() {
  if (!props.instanceName) return
  if (!confirm(`Delete '${props.instanceName}'?`)) return
  await invoke('delete_instance', { name: props.instanceName })
  emit('refresh')
  emit('status', `Deleted '${props.instanceName}'`)
}
</script>

<template>
  <main class="flex-1 flex flex-col overflow-hidden">
    <div v-if="!instance" class="flex-1 flex items-center justify-center">
      <div class="text-center">
        <p class="text-lg text-[#a0a5b4]">Welcome to MiaoMC</p>
        <p class="text-sm text-[#6b7080] mt-2">Select an instance or create a new one</p>
      </div>
    </div>

    <template v-else>
      <!-- Header Card -->
      <div class="m-4 p-4 rounded-xl bg-[#262a34] border border-[#2a2d36]">
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-xl font-bold text-white">{{ instance.name }}</h2>
            <div class="flex items-center gap-3 mt-1">
              <span class="text-sm text-[#78b4ff]">MC {{ instance.minecraft_version }}</span>
              <span v-if="instance.loader_type" class="text-sm text-[#50b450]">
                {{ instance.loader_type }} {{ instance.loader_version }}
              </span>
              <span v-else class="text-sm text-[#6b7080]">Vanilla</span>
            </div>
          </div>
          <div class="flex items-center gap-2">
            <button @click="deleteInstance" class="px-3 py-1.5 text-sm rounded-lg bg-[#3c2020] hover:bg-[#5c3030] text-[#f87171] transition">Delete</button>
            <button @click="launch" class="px-5 py-2 text-sm font-semibold rounded-lg bg-[#2d6b2d] hover:bg-[#3a8a3a] text-white transition shadow-lg shadow-green-900/20">
              Launch
            </button>
          </div>
        </div>
      </div>

      <!-- Tabs -->
      <div class="flex px-4 gap-1 border-b border-[#2a2d36]">
        <button
          v-for="tab in (['mods', 'resources', 'worlds', 'log'] as const)"
          :key="tab"
          @click="activeTab = tab"
          :class="[
            'px-4 py-2 text-sm capitalize transition-colors border-b-2',
            activeTab === tab
              ? 'text-[#78b4ff] border-[#78b4ff]'
              : 'text-[#6b7080] border-transparent hover:text-[#a0a5b4]'
          ]"
        >{{ tab }}</button>
      </div>

      <!-- Tab Content -->
      <div class="flex-1 overflow-y-auto p-4">
        <ModsTab v-if="activeTab === 'mods'" :instance-name="instanceName!" @status="s => $emit('status', s)" />
        <ResourcesTab v-else-if="activeTab === 'resources'" :instance-name="instanceName!" />
        <WorldsTab v-else-if="activeTab === 'worlds'" :instance-name="instanceName!" />
        <LogTab v-else-if="activeTab === 'log'" :instance-name="instanceName!" />
      </div>
    </template>
  </main>
</template>
