<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import {
  MiuixBasicComponent,
  MiuixBottomSheet,
  MiuixButton,
  MiuixCard,
  MiuixIcon,
  MiuixProgressIndicator,
  MiuixSwitchPreference,
} from 'miuix-vue'
import { Info, Refresh, Tune } from 'miuix-vue/icons'
import { Cli, type SoterSpoofState } from '../cli'
import { i18n } from '../i18n'
import { isDev } from '../utils/dev'

type LoadStatus = 'loading' | 'ready' | 'error'

const props = defineProps<{
  modelValue: boolean
  cli: Cli
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  changed: []
  notify: [message: string, error?: boolean]
}>()

const currentState = ref<SoterSpoofState | null>(null)
const stateStatus = ref<LoadStatus>('loading')
const stateError = ref('')
const desiredEnabled = ref(false)
const loadGeneration = ref(0)
const busy = ref(false)

function tr(key: string, fallback: string, ...args: unknown[]): string {
  const value = i18n.t(key, ...args)
  if (value !== key) return value
  let index = 0
  return fallback.replace(/%s/g, () => String(args[index++] ?? ''))
}

// The payload cannot be switched off at runtime, so the only meaningful action
// is a change of state; re-applying the current value would just ask the user
// to reboot for nothing.
const canApply = computed(() => stateStatus.value === 'ready'
  && currentState.value !== null
  && desiredEnabled.value !== currentState.value.enabled)

watch(() => props.modelValue, open => {
  if (open) load()
  else loadGeneration.value++
})

function isActive(generation: number): boolean {
  return generation === loadGeneration.value && props.modelValue
}

async function load(): Promise<void> {
  const generation = ++loadGeneration.value
  stateStatus.value = 'loading'
  stateError.value = ''
  await loadState(generation)
}

async function loadState(generation: number): Promise<void> {
  try {
    const state = isDev()
      ? ({ enabled: true, reboot_required: false } satisfies SoterSpoofState)
      : await props.cli.getSoterSpoofState()
    if (!isActive(generation)) return
    currentState.value = state
    desiredEnabled.value = state.enabled
    stateStatus.value = 'ready'
  } catch (error) {
    if (!isActive(generation)) return
    stateStatus.value = 'error'
    stateError.value = error instanceof Error ? error.message : String(error)
  }
}

function retryState(): void {
  if (busy.value || !props.modelValue) return
  stateStatus.value = 'loading'
  stateError.value = ''
  void loadState(loadGeneration.value)
}

function requestClose(): boolean {
  if (busy.value) return false
  emit('update:modelValue', false)
  return true
}

// App.vue owns the shared History stack. Expose the guarded close operation so
// a back/Escape request can ask this sheet to close without bypassing the busy
// protection used by the native bottom-sheet scrim.
defineExpose({ requestClose, busy })

async function apply(): Promise<void> {
  if (busy.value || !canApply.value) return

  busy.value = true
  try {
    const state = isDev()
      ? ({ enabled: desiredEnabled.value, reboot_required: true } satisfies SoterSpoofState)
      : await props.cli.setSoterSpoofEnabled(desiredEnabled.value)
    currentState.value = state
    emit('notify', tr(
      state.enabled ? 'prompt_soter_spoof_enabled' : 'prompt_soter_spoof_disabled',
      state.enabled
        ? 'Soter spoofing enabled. Reboot to apply.'
        : 'Soter spoofing disabled. Reboot to apply.',
    ))
    emit('changed')
    emit('update:modelValue', false)
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error)
    console.error('Unable to update Soter spoofing:', error)
    emit('notify', tr('prompt_soter_spoof_error', 'Unable to update Soter spoofing: %s', detail), true)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <MiuixBottomSheet
    :model-value="modelValue"
    :title="tr('soter_spoof_title', 'Spoof Soter attestation')"
    :allow-dismiss="!busy"
    :close-on-click-modal="!busy"
    @update:model-value="value => value ? undefined : requestClose()"
    @close="requestClose"
  >
    <div class="soter-sheet" :aria-busy="busy">
      <MiuixCard class="soter-card" press-feedback="none">
        <MiuixSwitchPreference
          v-model="desiredEnabled"
          :title="tr('soter_spoof_enable', 'Enable Soter spoofing')"
          :summary="tr('soter_zygisk_required', 'Install and enable a Zygisk loader separately. OMK does not bundle, download, or install one.')"
          :disabled="busy || stateStatus !== 'ready'"
        >
          <template #start><MiuixIcon :icon="Tune" :size="23" /></template>
        </MiuixSwitchPreference>
      </MiuixCard>

      <MiuixCard
        v-if="stateStatus === 'ready' && currentState !== null"
        class="soter-card current-config"
        press-feedback="none"
      >
        <MiuixBasicComponent :title="tr('soter_spoof_status', 'Current state')">
          <template #bottom>
            <div class="current-config-values">
              <strong>{{
                tr(
                  currentState.enabled ? 'soter_spoof_enabled' : 'soter_spoof_disabled',
                  currentState.enabled ? 'Enabled' : 'Disabled',
                )
              }}</strong>
              <span v-if="currentState.reboot_required">
                {{ tr('soter_spoof_reboot_required', 'Reboot the device to apply this change.') }}
              </span>
            </div>
          </template>
        </MiuixBasicComponent>
      </MiuixCard>

      <div v-if="stateStatus === 'loading'" class="soter-status" role="status">
        <MiuixProgressIndicator type="circular" :size="26" :stroke-width="2.5" />
        <span>{{ tr('soter_spoof_loading', 'Loading Soter spoofing state...') }}</span>
      </div>
      <div v-else-if="stateStatus === 'error'" class="soter-status soter-status--error" role="alert">
        <MiuixIcon :icon="Info" :size="24" />
        <strong>{{ tr('soter_spoof_load_error', 'Failed to load the Soter spoofing state') }}</strong>
        <span v-if="stateError">{{ stateError }}</span>
        <MiuixButton :disabled="busy" @click="retryState">
          <MiuixIcon :icon="Refresh" :size="18" />
          {{ tr('functional_button_retry', 'Retry') }}
        </MiuixButton>
      </div>

      <div class="sheet-actions">
        <MiuixButton :disabled="busy" @click="requestClose">
          {{ tr('functional_button_cancel', 'Cancel') }}
        </MiuixButton>
        <MiuixButton type="primary" :disabled="busy || !canApply" @click="apply">
          <MiuixProgressIndicator
            v-if="busy"
            type="circular"
            :size="19"
            :stroke-width="2.5"
          />
          {{ tr(busy ? 'soter_spoof_applying' : 'functional_button_apply', busy ? 'Applying...' : 'Apply') }}
        </MiuixButton>
      </div>
    </div>
  </MiuixBottomSheet>
</template>

<style scoped>
.soter-sheet {
  display: grid;
  gap: 12px;
  padding: 2px 0 max(18px, env(safe-area-inset-bottom));
}

.soter-card {
  flex: none;
}

.current-config-values {
  display: grid;
  gap: 3px;
  min-width: 0;
}

.current-config-values strong,
.current-config-values span {
  overflow-wrap: anywhere;
}

.current-config-values span {
  color: var(--m-color-on-surface-variant-summary);
  font-size: var(--m-text-body2-size);
}

.soter-status {
  min-height: 112px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 16px;
  text-align: center;
  color: var(--m-color-on-surface-variant-summary);
}

.soter-status--error {
  color: var(--m-color-error);
}

.soter-status .m-button {
  display: inline-flex;
  gap: 7px;
  margin-top: 4px;
}

.sheet-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  position: sticky;
  bottom: 0;
  padding-top: 8px;
  background: var(--m-color-background);
}

.sheet-actions .m-button {
  gap: 7px;
}
</style>
