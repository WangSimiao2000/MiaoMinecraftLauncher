<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{ instanceName: string }>()
const emit = defineEmits<{ status: [msg: string] }>()

interface ModItem { name: string; filename: string; enabled: boolean }
interface SearchHit { title: string; slug: string; downloads: number; description: string }

const mods = ref<ModItem[]>([])
const searching = ref(false)
const searchQuery = ref('')
const searchResults = ref<SearchHit[]>([])
const showSearch = ref(false)

async function loadMods() {
  mods.value = await invoke('get_mods', { instanceName: props.instanceName })
}

watch(() => props.instanceName, loadMods, { immediate: true })

async function toggle(filename: string) {
  await invoke('toggle_mod', { instanceName: props.instanceName, filename })
  await loadMods()
}

async function deleteMod(filename: string) {
  await invoke('delete_mod', { instanceName: props.instanceName, filename })
  await loadMods()
}

async function search() {
  if (!searchQuery.value) return
  searching.value = true
  const instances: any[] = await invoke('list_instances')
  const inst = instances.find(i => i.name === props.instanceName)
  if (!inst) return
  try {
    searchResults.value = await invoke('search_mods', {
      query: searchQuery.value,
      mcVersion: inst.minecraft_version,
      loader: inst.loader_type,
    })
  } catch (e: any) {
    emit('status', `Search failed: ${e}`)
  }
  searching.value = false
}

async function installMod(slug: string) {
  emit('status', `Installing ${slug}...`)
  try {
    const files: string[] = await invoke('install_mod', {
      instanceName: props.instanceName,
      projectId: slug,
    })
    emit('status', `Installed: ${files.join(', ')}`)
    await loadMods()
    showSearch.value = false
    searchResults.value = []
  } catch (e: any) {
    emit('status', `Install failed: ${e}`)
  }
}

function formatDl(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`
  if (n >= 1_000) return `${Math.floor(n / 1_000)}K`
  return n.toString()
}
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-4">
      <h3 class="text-sm font-semibold text-[#a0a5b4]">Mods ({{ mods.length }})</h3>
      <button @click="showSearch = !showSearch" class="px-3 py-1.5 text-xs rounded-lg bg-[#4b82c3]/20 text-[#78b4ff] hover:bg-[#4b82c3]/30 transition">
        {{ showSearch ? 'Close' : 'Search Modrinth' }}
      </button>
    </div>

    <!-- Inline Search -->
    <div v-if="showSearch" class="mb-4 p-4 rounded-xl bg-[#262a34] border border-[#2a2d36]">
      <div class="flex gap-2">
        <input
          v-model="searchQuery"
          @keyup.enter="search"
          placeholder="Search mods..."
          class="flex-1 px-3 py-2 text-sm rounded-lg bg-[#1a1d24] border border-[#2a2d36] text-white placeholder-[#6b7080] focus:border-[#4b82c3] focus:outline-none transition"
        />
        <button @click="search" :disabled="searching" class="px-4 py-2 text-sm rounded-lg bg-[#4b82c3] hover:bg-[#5a92d3] text-white transition disabled:opacity-50">
          {{ searching ? '...' : 'Search' }}
        </button>
      </div>
      <div v-if="searchResults.length" class="mt-3 space-y-2 max-h-60 overflow-y-auto">
        <button
          v-for="hit in searchResults"
          :key="hit.slug"
          @click="installMod(hit.slug)"
          class="w-full text-left p-3 rounded-lg bg-[#1a1d24] hover:bg-[#2d323c] transition"
        >
          <div class="flex justify-between items-center">
            <span class="text-sm font-medium text-white">{{ hit.title }}</span>
            <span class="text-xs text-[#6b7080]">{{ formatDl(hit.downloads) }}</span>
          </div>
          <p class="text-xs text-[#6b7080] mt-1 line-clamp-1">{{ hit.description }}</p>
        </button>
      </div>
    </div>

    <!-- Mods List -->
    <div v-if="mods.length === 0" class="text-center text-sm text-[#6b7080] py-8">
      <p>No mods installed</p>
      <p class="text-xs mt-1">Use 'Search Modrinth' to find and install mods</p>
    </div>
    <div v-else class="space-y-1">
      <div
        v-for="m in mods"
        :key="m.filename"
        class="flex items-center justify-between p-3 rounded-lg bg-[#262a34] hover:bg-[#2d323c] transition"
      >
        <div class="flex items-center gap-3">
          <button
            @click="toggle(m.filename)"
            :class="[
              'w-10 h-5 rounded-full relative transition-colors',
              m.enabled ? 'bg-[#50b450]' : 'bg-[#3c3c3c]'
            ]"
          >
            <span :class="[
              'absolute top-0.5 w-4 h-4 rounded-full bg-white shadow transition-transform',
              m.enabled ? 'left-5' : 'left-0.5'
            ]" />
          </button>
          <span :class="['text-sm', m.enabled ? 'text-white' : 'text-[#6b7080]']">{{ m.name }}</span>
        </div>
        <button @click="deleteMod(m.filename)" class="text-xs text-[#6b7080] hover:text-[#f87171] transition">Del</button>
      </div>
    </div>
  </div>
</template>
