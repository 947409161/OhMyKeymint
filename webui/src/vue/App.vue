<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue'
import {
  MiuixDialog,
  MiuixRadioButtonPreference,
  MiuixIcon,
  MiuixButton,
  MiuixNavigationBar,
  MiuixProgressIndicator,
  MiuixSnackbarHost,
  showSnackbar,
} from 'miuix-vue'
import { All, Settings, Tune } from 'miuix-vue/icons'
import { afterPaint } from '../utils/after_paint'
import {
  Cli,
  type ActivityEntry,
  type DiagnosticsState,
  type KeyboxInspector,
  type KeyboxRevocationStatus,
  type RkpIssuedArtifact,
  type RkpProtocol,
} from '../cli'
import { FileSelector } from '../file_selector/file_selector'
import { History } from '../history'
import { i18n } from '../i18n'
import { fetchLatestSecurityPatch } from '../security_patch'
import { isDev } from '../utils/dev'
import HomeView, { type KeyboxStatus, type TeeStatus } from './HomeView.vue'
import PifFingerprintDialog from './PifFingerprintDialog.vue'
import SettingsView from './SettingsView.vue'
import SoterDialog from './SoterDialog.vue'
import ToolsView, { type ToolEvent } from './ToolsView.vue'
import FileBrowserSheet from './FileBrowserSheet.vue'
import './app.scss'

const cli = new Cli()
const fileSelector = new FileSelector()
const history = new History()
const pageScroller = document.getElementById('app')!

const pageIndex = ref(0)
const pageDirection = ref(1)
const pageScrollPositions = [0, 0, 0]
const keyboxStatus = ref<KeyboxStatus>('loading')
const keyboxSource = ref<'google_hardware' | 'google_remote' | 'unknown'>('unknown')
const keyboxLevel = ref<'tee' | 'strongbox' | 'unknown'>('unknown')
const keyboxRevocation = ref<KeyboxRevocationStatus>('not_checked')
const keyboxInspector = shallowRef<KeyboxInspector | null>(null)
const diagnostics = shallowRef<DiagnosticsState | null>(null)
const diagnosticsStatus = ref<'loading' | 'ready' | 'error'>('loading')
const teeStatus = ref<TeeStatus>('loading')
const securityPatch = ref<string | null>(null)
const spoofedDevice = ref<string | null | undefined>(undefined)
const activities = ref<ActivityEntry[]>([])
const activityStatus = ref<'loading' | 'ready' | 'error'>('loading')
const activityClearBusy = ref(false)
const securityPatchBusy = ref<'sync' | 'restore' | null>(null)
const maintenanceBusy = ref<'gms' | 'rkp' | null>(null)
const rkpOpen = ref(false)
const rkpError = ref('')
const rkpIssueOpen = ref(false)
const rkpIssueConfirmOpen = ref(false)
const rkpImportConfirmOpen = ref(false)
const rkpIssueBusy = ref(false)
const rkpProtocol = ref<RkpProtocol>('auto')
const rkpProtocols: RkpProtocol[] = ['auto', 'v2', 'v1', 'v1-native']
const rkpListBusy = ref(false)
const rkpIssuanceBusy = computed(() => rkpIssueBusy.value || rkpListBusy.value)
const rkpIssueError = ref('')
const rkpIssuedArtifacts = ref<RkpIssuedArtifact[]>([])
const pendingRkpImport = ref<RkpIssuedArtifact | null>(null)
const soterOpen = ref(false)
const pifOpen = ref(false)
const keyboxOpen = ref(false)
const selectedKeybox = ref<{ name: string, contents: Uint8Array } | null>(null)
const keyboxBusy = ref(false)
const settingsView = ref<InstanceType<typeof SettingsView> | null>(null)
const pifDialog = ref<InstanceType<typeof PifFingerprintDialog> | null>(null)
const soterDialog = ref<InstanceType<typeof SoterDialog> | null>(null)

const pageIds = ['home', 'tools', 'settings'] as const
const navItems = computed(() => [
  { label: i18n.t('nav_home') === 'nav_home' ? 'Home' : i18n.t('nav_home') },
  { label: i18n.t('nav_tools') === 'nav_tools' ? 'Tools' : i18n.t('nav_tools') },
  { label: i18n.t('nav_settings') === 'nav_settings' ? 'Settings' : i18n.t('nav_settings') },
])

let unsubscribeFileSelector: (() => void) | null = null
let keydownListener: ((event: KeyboardEvent) => void) | null = null
let pageHistoryActive = false
const overlayHistory = new Set<string>()
let navigationElement: HTMLElement | null = null
let navigationPointerId: number | null = null
let navigationPointerStartX = 0
let navigationPointerStartIndex = 0
let navigationPointerSlotWidth = 1
let navigationPointerLeft = 0
let navigationPointerWidth = 1
let navigationPointerX = 0
let navigationFrame: number | null = null
let navigationWasDragged = false
let navigationSuppressClick = false

// KernelSU's floating bar keeps its drag/spring interaction independent from
// the optional liquid-glass surface treatment.  The same gesture therefore
// remains available when the bar is rendered as a regular floating surface.
function floatingNavigationEnabled(): boolean {
  const root = document.documentElement
  return root.dataset.floatingBottomBar === 'true'
}

function clampNavigationIndex(index: number): number {
  return Math.max(0, Math.min(pageIds.length - 1, index))
}

function onNavigationPointerDown(event: PointerEvent): void {
  if (!navigationElement || !floatingNavigationEnabled() || !event.isPrimary || event.button !== 0
      || navigationPointerId !== null) return
  const bounds = navigationElement.getBoundingClientRect()
  navigationPointerLeft = bounds.left
  navigationPointerWidth = Math.max(1, bounds.width)
  navigationPointerSlotWidth = Math.max(
    1,
    (bounds.width - 8) / pageIds.length,
  )
  navigationPointerId = event.pointerId
  navigationPointerStartX = event.clientX
  navigationPointerX = event.clientX
  navigationPointerStartIndex = Math.floor(clampNavigationIndex(
    (event.clientX - navigationPointerLeft - 4) / navigationPointerSlotWidth,
  ))
  navigationWasDragged = false
  navigationElement.classList.add('is-pressing')
  navigationElement.style.setProperty('--omk-liquid-nav-index', String(navigationPointerStartIndex))
  navigationElement.style.setProperty('--omk-liquid-nav-scale', '1.392857')
  navigationElement.style.setProperty('--omk-liquid-nav-panel-offset', '0px')
  navigationElement.style.setProperty(
    '--omk-liquid-nav-highlight-x',
    `${event.clientX - bounds.left}px`,
  )
  try { navigationElement.setPointerCapture(event.pointerId) } catch { /* WebView may reject capture. */ }
  event.preventDefault()
}

function onNavigationPointerMove(event: PointerEvent): void {
  if (!navigationElement || navigationPointerId !== event.pointerId) return
  navigationPointerX = event.clientX
  if (Math.abs(event.clientX - navigationPointerStartX) > 1) {
    navigationWasDragged = true
  }
  // A gesture only paints once per display frame. Use the geometry captured
  // before pressing, not a layout read after moving the floating surface.
  if (navigationFrame === null) navigationFrame = window.requestAnimationFrame(paintNavigationDrag)
  event.preventDefault()
}

function paintNavigationDrag(): void {
  navigationFrame = null
  if (!navigationElement || navigationPointerId === null) return
  if (navigationWasDragged) navigationElement.classList.add('is-dragging')
  const position = navigationPointerStartIndex
    + (navigationPointerX - navigationPointerStartX) / navigationPointerSlotWidth
  navigationElement.style.setProperty('--omk-liquid-nav-index', String(clampNavigationIndex(position)))
  const distance = navigationPointerX - navigationPointerStartX
  const fraction = Math.min(1, Math.abs(distance) / navigationPointerWidth)
  const rubberBand = Math.sign(distance) * 4 * (1 - (1 - fraction) ** 2)
  navigationElement.style.setProperty('--omk-liquid-nav-panel-offset', `${rubberBand}px`)
  navigationElement.style.setProperty(
    '--omk-liquid-nav-highlight-x',
    `${navigationPointerX - navigationPointerLeft}px`,
  )
}

function onNavigationPointerUp(event: PointerEvent): void {
  if (!navigationElement || navigationPointerId !== event.pointerId) return
  if (navigationFrame !== null) window.cancelAnimationFrame(navigationFrame)
  navigationFrame = null
  const target = navigationWasDragged
    ? Math.round(clampNavigationIndex(navigationPointerStartIndex
      + (event.clientX - navigationPointerStartX) / navigationPointerSlotWidth))
    : navigationPointerStartIndex
  navigationSuppressClick = navigationWasDragged
  navigationPointerId = null
  navigationWasDragged = false
  navigationElement.classList.remove('is-pressing', 'is-dragging')
  navigationElement.style.setProperty('--omk-liquid-nav-scale', '1')
  navigationElement.style.setProperty('--omk-liquid-nav-panel-offset', '0px')
  navigationElement.style.setProperty('--omk-liquid-nav-highlight-x', '50%')
  navigationElement.style.setProperty('--omk-liquid-nav-index', String(target))
  try { navigationElement.releasePointerCapture(event.pointerId) } catch { /* Already released. */ }
  if (target !== pageIndex.value) setPage(target)
  window.setTimeout(() => { navigationSuppressClick = false }, 120)
  event.preventDefault()
}

function onNavigationClickCapture(event: Event): void {
  if (!navigationSuppressClick) return
  navigationSuppressClick = false
  event.preventDefault()
  event.stopImmediatePropagation()
}

function onNavigationPointerCancel(event: PointerEvent): void {
  if (!navigationElement || navigationPointerId !== event.pointerId) return
  if (navigationFrame !== null) window.cancelAnimationFrame(navigationFrame)
  navigationFrame = null
  navigationPointerId = null
  navigationWasDragged = false
  navigationSuppressClick = false
  navigationElement.classList.remove('is-pressing', 'is-dragging')
  navigationElement.style.setProperty('--omk-liquid-nav-scale', '1')
  navigationElement.style.setProperty('--omk-liquid-nav-panel-offset', '0px')
  navigationElement.style.setProperty('--omk-liquid-nav-highlight-x', '50%')
  navigationElement.style.setProperty('--omk-liquid-nav-index', String(pageIndex.value))
}

function notify(message: string, error = false): void {
  void showSnackbar({ message, duration: error ? 6000 : 'long', withDismissAction: true })
}

function setPage(index: number): void {
  if (index < 0 || index >= pageIds.length || index === pageIndex.value) return
  pageIndex.value = index
  if (index !== 0 && !pageHistoryActive) {
    pageHistoryActive = true
    history.push('main-page', () => {
      pageHistoryActive = false
      pageIndex.value = 0
    })
  } else if (index === 0 && pageHistoryActive) {
    pageHistoryActive = false
    history.consume('main-page')
  }
}

function onSettingsOverlayOpen(): void {
  const key = 'settings-overlay'
  if (overlayHistory.has(key)) return
  overlayHistory.add(key)
  history.push(key, () => {
    overlayHistory.delete(key)
    settingsView.value?.dismissOverlay()
  })
}

function onSettingsOverlayClose(): void {
  const key = 'settings-overlay'
  if (overlayHistory.delete(key)) history.consume(key)
}

function handleEscape(): void {
  if (soterOpen.value && soterDialog.value?.busy) return
  if (rkpIssueOpen.value && rkpIssueBusy.value) return
  if (history.size > 0) history.back()
}

async function refreshIdentity(force = false): Promise<void> {
  // The refresh path always queries the current backend state; keep the
  // parameter for callers that request an explicit refresh after a mutation.
  void force
  if (isDev()) {
    keyboxStatus.value = 'custom'
    keyboxSource.value = 'google_remote'
    keyboxLevel.value = 'tee'
    keyboxRevocation.value = 'not_listed'
    keyboxInspector.value = {
      valid: true,
      bundled: false,
      rsa: null,
      ec: {
        algorithm: 'EC',
        chain_length: 3,
        serials: ['01', '02', '03'],
        leaf_subject: 'CN=Demo keybox',
        leaf_issuer: 'CN=Demo issuer',
        valid_from: '2026-01-01T00:00:00Z',
        valid_until: '2036-01-01T00:00:00Z',
        certificates: [
          { serial: '01', subject: 'CN=Demo keybox', issuer: 'CN=Demo issuer', valid_from: '2026-01-01T00:00:00Z', valid_until: '2036-01-01T00:00:00Z' },
          { serial: '02', subject: 'CN=Demo issuer', issuer: 'CN=Demo root', valid_from: '2025-01-01T00:00:00Z', valid_until: '2040-01-01T00:00:00Z' },
          { serial: '03', subject: 'CN=Demo root', issuer: 'CN=Demo root', valid_from: '2024-01-01T00:00:00Z', valid_until: '2044-01-01T00:00:00Z' },
        ],
      },
    }
    teeStatus.value = 'normal'
    securityPatch.value = '2026-08-01'
    spoofedDevice.value = 'Google Pixel 9 Pro'
    return
  }
  try {
    await afterPaint()
    const [keybox, patch, tee, pif] = await Promise.allSettled([
      cli.getKeyboxState(),
      cli.getSystemSecurityPatch(),
      cli.getTeeStatus(),
      cli.getPifFingerprintState(),
    ])
    if (keybox.status === 'fulfilled') {
      const value = keybox.value
      keyboxStatus.value = value.valid ? (value.bundled ? 'bundled' : 'custom') : 'invalid'
      keyboxSource.value = value.source
      keyboxLevel.value = value.level
      keyboxRevocation.value = value.valid ? 'checking' : value.revocation
    } else keyboxStatus.value = 'error'
    if (patch.status === 'fulfilled') securityPatch.value = patch.value
    if (tee.status === 'fulfilled') teeStatus.value = 'normal'
    else teeStatus.value = 'error'
    if (pif.status === 'fulfilled') {
      spoofedDevice.value = pif.value.enabled
        ? (/^google\s/i.test(pif.value.model) ? pif.value.model : `Google ${pif.value.model}`)
        : null
    }
    // Network revocation checks must not delay the other local status fields.
    if (keybox.status === 'fulfilled' && keybox.value.valid) {
      try { keyboxRevocation.value = await cli.checkKeyboxRevocation() }
      catch { keyboxRevocation.value = 'unknown' }
    }
  } catch (error) {
    console.error('Unable to load OMK identity:', error)
  }
}

async function refreshDiagnostics(): Promise<void> {
  diagnosticsStatus.value = 'loading'
  if (isDev()) {
    diagnostics.value = {
      keymint: { status: 'running', pid: 1234 },
      keystore2: { status: 'running', pid: 1240 },
      injector: { status: 'running', pid: 1250 },
      tee: { status: 'available', version: 300, name: 'TEE KeyMint' },
      strongbox: { status: 'unavailable', version: null, name: null },
      rkp_tee: { status: 'available', pid: null },
      rkp_strongbox: { status: 'unavailable', pid: null },
      selinux: 'enforcing',
    }
    diagnosticsStatus.value = 'ready'
    return
  }

  await afterPaint()
  const [inspector, state] = await Promise.allSettled([
    cli.getKeyboxInspector(),
    cli.getDiagnostics(),
  ])
  if (inspector.status === 'fulfilled') keyboxInspector.value = inspector.value
  if (state.status === 'fulfilled') {
    diagnostics.value = state.value
    diagnosticsStatus.value = 'ready'
  } else {
    diagnosticsStatus.value = 'error'
    console.error('Unable to load OMK service diagnostics:', state.reason)
  }
}

async function refreshActivity(): Promise<void> {
  try {
    activities.value = (await cli.getActivityLog()).slice().sort((left, right) => right.timestamp - left.timestamp)
    activityStatus.value = 'ready'
  } catch (error) {
    if (isDev()) {
      const now = Math.floor(Date.now() / 1000)
      activities.value = [{ action: 'keybox_changed', detail: '', timestamp: now - 3600 }]
      activityStatus.value = 'ready'
    } else {
      activityStatus.value = 'error'
      console.error('Unable to load activity:', error)
    }
  }
}

async function chooseKeybox(): Promise<void> {
  try {
    const selected = await fileSelector.getSystemFileContent('xml')
    if (selected) {
      selectedKeybox.value = selected
      keyboxOpen.value = true
    }
  } catch (error) {
    notify(error instanceof Error ? error.message : String(error), true)
  }
}

async function installKeybox(): Promise<void> {
  const selected = selectedKeybox.value
  if (!selected || keyboxBusy.value) return
  keyboxBusy.value = true
  try {
    await afterPaint()
    if (!isDev()) await cli.installKeybox(selected.contents)
    notify(i18n.t('prompt_keybox_replaced'))
    keyboxOpen.value = false
    selectedKeybox.value = null
    await Promise.all([refreshIdentity(true), refreshDiagnostics(), refreshActivity()])
  } catch (error) {
    notify(i18n.t('prompt_keybox_replace_error', error instanceof Error ? error.message : String(error)), true)
  } finally {
    keyboxBusy.value = false
  }
}

async function syncPatch(restore: boolean): Promise<void> {
  if (securityPatchBusy.value !== null) return
  securityPatchBusy.value = restore ? 'restore' : 'sync'
  try {
    await afterPaint()
    if (restore) {
      if (!isDev()) await cli.restoreDefaultSecurityPatch()
      await refreshIdentity(true)
      notify(i18n.t('prompt_security_patch_restored_default'))
    } else {
      const date = await fetchLatestSecurityPatch(() => cli.fetchSecurityBulletin())
      const applied = isDev() ? date : await cli.syncSecurityPatch(date)
      securityPatch.value = applied
      notify(i18n.t('prompt_security_patch_sync_complete', applied))
    }
    await refreshActivity()
  } catch (error) {
    notify(error instanceof Error ? error.message : String(error), true)
  } finally {
    securityPatchBusy.value = null
  }
}

async function clearGmsCache(): Promise<void> {
  if (maintenanceBusy.value !== null) return
  maintenanceBusy.value = 'gms'
  try {
    await afterPaint()
    if (isDev()) {
      notify(i18n.t('maintenance_preview_only'))
      return
    }
    await cli.clearGmsCache()
    notify(i18n.t('gms_cache_cleared'))
  } catch (error) {
    notify(error instanceof Error ? error.message : String(error), true)
  } finally {
    maintenanceBusy.value = null
  }
}

async function repairRkp(): Promise<void> {
  if (maintenanceBusy.value !== null) return
  maintenanceBusy.value = 'rkp'
  rkpError.value = ''
  try {
    await afterPaint()
    if (isDev()) {
      notify(i18n.t('maintenance_preview_only'))
      return
    }
    await cli.repairRkp()
    notify(i18n.t('rkp_tool_completed'))
    rkpOpen.value = false
  } catch (error) {
    rkpError.value = error instanceof Error ? error.message : String(error)
    notify(rkpError.value, true)
  } finally {
    maintenanceBusy.value = null
  }
}

async function refreshRkpIssuedArtifacts(): Promise<void> {
  if (rkpListBusy.value) return
  if (isDev()) {
    rkpIssuedArtifacts.value = []
    return
  }
  rkpListBusy.value = true
  try {
    rkpIssuedArtifacts.value = await cli.listIssuedRkpKeyboxes()
    rkpIssueError.value = ''
  } catch (error) {
    rkpIssueError.value = error instanceof Error ? error.message : String(error)
  } finally {
    rkpListBusy.value = false
  }
}

async function openRkpIssue(): Promise<void> {
  if (rkpIssuanceBusy.value) return
  rkpIssueOpen.value = true
  rkpIssueError.value = ''
  await afterPaint()
  await refreshRkpIssuedArtifacts()
}

async function issueRkpKeybox(): Promise<void> {
  if (rkpIssuanceBusy.value || !rkpIssueConfirmOpen.value) return
  rkpIssueBusy.value = true
  rkpIssueError.value = ''
  try {
    await afterPaint()
    if (isDev()) {
      notify(i18n.t('maintenance_preview_only'))
      rkpIssueConfirmOpen.value = false
      return
    }
    const artifact = await cli.issueRkpKeybox(rkpProtocol.value)
    rkpIssuedArtifacts.value = [artifact, ...rkpIssuedArtifacts.value.filter(item => item.id !== artifact.id)]
    notify(i18n.t('rkp_issue_completed'))
    rkpIssueConfirmOpen.value = false
  } catch (error) {
    rkpIssueError.value = error instanceof Error ? error.message : String(error)
    notify(rkpIssueError.value, true)
  } finally {
    rkpIssueBusy.value = false
  }
}

function requestRkpImport(artifact: RkpIssuedArtifact): void {
  if (rkpIssuanceBusy.value) return
  rkpIssueError.value = ''
  pendingRkpImport.value = artifact
  rkpImportConfirmOpen.value = true
}

async function importRkpKeybox(): Promise<void> {
  const artifact = pendingRkpImport.value
  if (!artifact || rkpIssuanceBusy.value || !rkpImportConfirmOpen.value) return
  rkpIssueBusy.value = true
  rkpIssueError.value = ''
  try {
    await afterPaint()
    if (isDev()) {
      notify(i18n.t('maintenance_preview_only'))
      rkpImportConfirmOpen.value = false
      pendingRkpImport.value = null
      return
    }
    await cli.importIssuedRkpKeybox(artifact.id)
    notify(i18n.t('rkp_import_completed'))
    rkpImportConfirmOpen.value = false
    pendingRkpImport.value = null
    await refreshRkpIssuedArtifacts()
    await refreshIdentity(true)
  } catch (error) {
    rkpIssueError.value = error instanceof Error ? error.message : String(error)
    notify(rkpIssueError.value, true)
  } finally {
    rkpIssueBusy.value = false
  }
}

function formatRkpCreatedAt(timestamp: number): string {
  return new Date(timestamp * 1000).toLocaleString(i18n.lang)
}

function onTool(event: ToolEvent): void {
  switch (event) {
    case 'installKeybox': void chooseKeybox(); break
    case 'syncSecurityPatch': void syncPatch(false); break
    case 'restoreSecurityPatch': void syncPatch(true); break
    case 'openSoterBeta': soterOpen.value = true; break
    case 'spoofPif': pifOpen.value = true; break
    case 'clearGmsCache': void clearGmsCache(); break
    case 'repairRkp': rkpError.value = ''; rkpOpen.value = true; break
    case 'issueRkp': void openRkpIssue(); break
  }
}

async function clearActivities(): Promise<void> {
  if (activityClearBusy.value) return
  activityClearBusy.value = true
  try {
    if (!isDev()) await cli.clearActivityLog()
    activities.value = []
    notify(i18n.t('home_activity_cleared'))
  } catch (error) {
    notify(error instanceof Error ? error.message : String(error), true)
  } finally {
    activityClearBusy.value = false
  }
}

onMounted(async () => {
  navigationElement = document.querySelector<HTMLElement>('.main-navigation')
  navigationElement?.style.setProperty('--omk-liquid-nav-index', String(pageIndex.value))
  navigationElement?.style.setProperty('--omk-liquid-nav-scale', '1')
  navigationElement?.addEventListener('pointerdown', onNavigationPointerDown)
  navigationElement?.addEventListener('pointermove', onNavigationPointerMove)
  navigationElement?.addEventListener('pointerup', onNavigationPointerUp)
  navigationElement?.addEventListener('pointercancel', onNavigationPointerCancel)
  navigationElement?.addEventListener('click', onNavigationClickCapture, true)
  keydownListener = event => {
    if (event.key === 'Escape') {
      event.preventDefault()
      handleEscape()
    }
  }
  window.addEventListener('keydown', keydownListener)
  unsubscribeFileSelector = fileSelector.subscribe(() => {
    if (fileSelector.open && !overlayHistory.has('file-selector')) {
      overlayHistory.add('file-selector')
      history.push('file-selector', () => fileSelector.cancel())
    } else if (!fileSelector.open && overlayHistory.delete('file-selector')) {
      history.consume('file-selector')
    }
  })
  // Package enumeration is only needed when the picker opens. In particular,
  // do not call the synchronous Android package bridge during home startup.
  await Promise.all([refreshIdentity(), refreshDiagnostics(), refreshActivity()])
})

onBeforeUnmount(() => {
  if (navigationFrame !== null) window.cancelAnimationFrame(navigationFrame)
  navigationFrame = null
  if (keydownListener !== null) window.removeEventListener('keydown', keydownListener)
  navigationElement?.removeEventListener('pointerdown', onNavigationPointerDown)
  navigationElement?.removeEventListener('pointermove', onNavigationPointerMove)
  navigationElement?.removeEventListener('pointerup', onNavigationPointerUp)
  navigationElement?.removeEventListener('pointercancel', onNavigationPointerCancel)
  navigationElement?.removeEventListener('click', onNavigationClickCapture, true)
  navigationElement = null
  unsubscribeFileSelector?.()
  history.destroy()
})

watch(pageIndex, (index, previousIndex) => {
  pageScrollPositions[previousIndex] = pageScroller.scrollTop
  pageDirection.value = index > previousIndex ? 1 : -1
  void nextTick(() => {
    if (pageIndex.value === index) {
      pageScroller.scrollTo(0, pageScrollPositions[index] ?? 0)
    }
  })
  if (navigationPointerId === null) {
    navigationElement?.style.setProperty('--omk-liquid-nav-index', String(index))
  }
})

watch(pifOpen, open => {
  if (open && !overlayHistory.has('pif-fingerprint')) {
    overlayHistory.add('pif-fingerprint')
    history.push('pif-fingerprint', () => {
      overlayHistory.delete('pif-fingerprint')
      pifDialog.value?.requestClose()
    })
  } else if (!open && overlayHistory.delete('pif-fingerprint')) history.consume('pif-fingerprint')
})
function trackSoterOverlay(): void {
  const key = 'soter-beta'
  if (!soterOpen.value || overlayHistory.has(key)) return
  overlayHistory.add(key)
  history.push(key, () => {
    overlayHistory.delete(key)
    if (soterDialog.value?.requestClose() === false) {
      // Re-arm after the current popstate handler finishes so a busy dialog
      // does not lose its back entry or close the page beneath it.
      void nextTick(trackSoterOverlay)
    }
  })
}
watch(soterOpen, open => {
  if (open) trackSoterOverlay()
  else if (overlayHistory.delete('soter-beta')) history.consume('soter-beta')
})
watch(keyboxOpen, open => {
  if (open && !overlayHistory.has('keybox')) {
    overlayHistory.add('keybox')
    history.push('keybox', () => { if (!keyboxBusy.value) keyboxOpen.value = false })
  } else if (!open && overlayHistory.delete('keybox')) history.consume('keybox')
})
function trackRkpOverlay(): void {
  if (!rkpOpen.value || overlayHistory.has('rkp')) return
  overlayHistory.add('rkp')
  history.push('rkp', () => {
    overlayHistory.delete('rkp')
    if (maintenanceBusy.value === 'rkp') void nextTick(trackRkpOverlay)
    else rkpOpen.value = false
  })
}
watch(rkpOpen, open => {
  if (open) trackRkpOverlay()
  else if (overlayHistory.delete('rkp')) history.consume('rkp')
})
function trackRkpIssuanceOverlay(key: 'rkp-issuance' | 'rkp-issue-confirm' | 'rkp-import-confirm', open: boolean): void {
  if (!open || overlayHistory.has(key)) return
  overlayHistory.add(key)
  history.push(key, () => {
    overlayHistory.delete(key)
    if (rkpIssueBusy.value) void nextTick(() => trackRkpIssuanceOverlay(key, true))
    else if (key === 'rkp-issuance') rkpIssueOpen.value = false
    else if (key === 'rkp-issue-confirm') rkpIssueConfirmOpen.value = false
    else rkpImportConfirmOpen.value = false
  })
}
watch(rkpIssueOpen, open => {
  if (open) trackRkpIssuanceOverlay('rkp-issuance', true)
  else if (overlayHistory.delete('rkp-issuance')) history.consume('rkp-issuance')
})
watch(rkpIssueConfirmOpen, open => {
  if (open) trackRkpIssuanceOverlay('rkp-issue-confirm', true)
  else if (overlayHistory.delete('rkp-issue-confirm')) history.consume('rkp-issue-confirm')
})
watch(rkpImportConfirmOpen, open => {
  if (open) trackRkpIssuanceOverlay('rkp-import-confirm', true)
  else if (overlayHistory.delete('rkp-import-confirm')) history.consume('rkp-import-confirm')
})
</script>

<template>
  <div class="omk-app">
    <main class="page-host" :style="{ '--omk-page-direction': pageDirection }">
      <Transition name="page-switch">
        <HomeView
          v-show="pageIndex === 0"
          :keybox-status="keyboxStatus"
          :keybox-source="keyboxSource"
          :keybox-level="keyboxLevel"
          :keybox-revocation="keyboxRevocation"
          :keybox-inspector="keyboxInspector"
          :diagnostics="diagnostics"
          :diagnostics-status="diagnosticsStatus"
          :tee-status="teeStatus"
          :security-patch="securityPatch"
          :spoofed-device="spoofedDevice"
          :activities="activities"
          :activity-status="activityStatus"
          :activity-clear-busy="activityClearBusy"
          @refresh-diagnostics="refreshDiagnostics"
          @clear-activities="clearActivities"
        />
      </Transition>
      <Transition name="page-switch">
        <ToolsView
          v-show="pageIndex === 1"
          :security-patch-busy="securityPatchBusy"
          :maintenance-busy="maintenanceBusy"
          :rkp-issuance-busy="rkpIssuanceBusy"
          @install-keybox="onTool('installKeybox')"
          @sync-security-patch="onTool('syncSecurityPatch')"
          @restore-security-patch="onTool('restoreSecurityPatch')"
          @open-soter-beta="onTool('openSoterBeta')"
          @spoof-pif="onTool('spoofPif')"
          @clear-gms-cache="onTool('clearGmsCache')"
          @repair-rkp="onTool('repairRkp')"
          @issue-rkp="onTool('issueRkp')"
        />
      </Transition>
      <Transition name="page-switch">
        <SettingsView
          v-show="pageIndex === 2"
          ref="settingsView"
          @overlay-open="onSettingsOverlayOpen"
          @overlay-close="onSettingsOverlayClose"
        />
      </Transition>
    </main>

    <Teleport to="body">
      <div class="navigation-dock">
        <MiuixNavigationBar
          :model-value="pageIndex"
          :items="navItems"
          :data-active-index="pageIndex"
          class="main-navigation"
          @update:model-value="setPage"
        >
          <template #icon="{ index }">
            <MiuixIcon :icon="index === 0 ? All : index === 1 ? Tune : Settings" :size="24" />
          </template>
        </MiuixNavigationBar>
      </div>
    </Teleport>

    <FileBrowserSheet :selector="fileSelector" />

    <MiuixDialog
      v-model="keyboxOpen"
      :title="i18n.t('replace_keybox_title')"
      :close-on-click-modal="!keyboxBusy"
    >
      <template #default="{ close }">
        <div class="confirm-sheet">
          <p>{{ selectedKeybox ? i18n.t('replace_keybox_selected_file', selectedKeybox.name) : '' }}</p>
          <div class="confirm-actions">
            <MiuixButton :disabled="keyboxBusy" @click="close">
              {{ i18n.t('functional_button_cancel') }}
            </MiuixButton>
            <MiuixButton type="primary" :disabled="keyboxBusy" @click="installKeybox">
              <MiuixProgressIndicator v-if="keyboxBusy" type="circular" :size="18" />
              {{ i18n.t('functional_button_replace') }}
            </MiuixButton>
          </div>
        </div>
      </template>
    </MiuixDialog>

    <PifFingerprintDialog
      ref="pifDialog"
      v-model="pifOpen"
      :cli="cli"
      @notify="notify"
      @changed="refreshIdentity(true); refreshActivity()"
    />
    <MiuixDialog
      :model-value="rkpOpen"
      :title="i18n.t('tools_repair_rkp')"
      :close-on-click-modal="maintenanceBusy !== 'rkp'"
      @update:model-value="value => { if (maintenanceBusy !== 'rkp') rkpOpen = value }"
    >
      <div class="confirm-sheet">
        <p>{{ i18n.t('rkp_repair_warning') }}</p>
        <p v-if="rkpError" role="alert">{{ rkpError }}</p>
        <div class="confirm-actions">
          <MiuixButton :disabled="maintenanceBusy === 'rkp'" @click="rkpOpen = false">
            {{ i18n.t('functional_button_cancel') }}
          </MiuixButton>
          <MiuixButton type="primary" :disabled="maintenanceBusy !== null" @click="repairRkp">
            <MiuixProgressIndicator v-if="maintenanceBusy === 'rkp'" type="circular" :size="18" />
            {{ i18n.t('rkp_repair_confirm') }}
          </MiuixButton>
        </div>
      </div>
    </MiuixDialog>
    <MiuixDialog
      :model-value="rkpIssueOpen"
      :title="i18n.t('tools_issue_rkp')"
      :close-on-click-modal="!rkpIssueBusy"
      @update:model-value="value => { if (!rkpIssueBusy && !rkpIssueConfirmOpen && !rkpImportConfirmOpen) rkpIssueOpen = value }"
    >
      <div class="confirm-sheet" :aria-busy="rkpIssuanceBusy">
        <p>{{ i18n.t('rkp_issue_description') }}</p>
        <div class="rkp-protocol-options" role="radiogroup" :aria-label="i18n.t('rkp_protocol')">
          <MiuixRadioButtonPreference
            v-for="protocol in rkpProtocols"
            :key="protocol"
            :model-value="rkpProtocol === protocol"
            :title="i18n.t(`rkp_protocol_${protocol}`)"
            :disabled="rkpIssuanceBusy"
            @select="rkpProtocol = protocol"
          />
        </div>
        <p v-if="rkpIssueError" role="alert">{{ rkpIssueError }}</p>
        <p v-if="rkpListBusy" class="rkp-loading" role="status">
          <MiuixProgressIndicator type="circular" :size="18" />
          {{ i18n.t('rkp_list_loading') }}
        </p>
        <div v-if="rkpIssuedArtifacts.length" class="rkp-issued-list">
          <article v-for="artifact in rkpIssuedArtifacts" :key="artifact.id" class="rkp-issued-item">
            <div class="rkp-issued-item__metadata">
              <strong>{{ formatRkpCreatedAt(artifact.created_at) }}</strong>
              <span v-for="(subject, index) in artifact.chain_subjects" :key="`${artifact.id}-${index}`">{{ subject }}</span>
            </div>
            <MiuixButton :disabled="rkpIssuanceBusy" @click="requestRkpImport(artifact)">
              {{ i18n.t('rkp_import_button') }}
            </MiuixButton>
          </article>
        </div>
        <p v-else-if="!rkpListBusy && !rkpIssueError">{{ i18n.t('rkp_issued_empty') }}</p>
        <div class="confirm-actions">
          <MiuixButton :disabled="rkpIssuanceBusy" @click="refreshRkpIssuedArtifacts">
            {{ i18n.t('rkp_refresh_list') }}
          </MiuixButton>
          <MiuixButton type="primary" :disabled="rkpIssuanceBusy" @click="rkpIssueConfirmOpen = true">
            <MiuixProgressIndicator v-if="rkpIssueBusy" type="circular" :size="18" />
            {{ i18n.t('rkp_issue_button') }}
          </MiuixButton>
        </div>
      </div>
    </MiuixDialog>
    <MiuixDialog
      :model-value="rkpIssueConfirmOpen"
      :title="i18n.t('rkp_issue_confirm_title')"
      :close-on-click-modal="!rkpIssueBusy"
      @update:model-value="value => { if (!rkpIssueBusy) rkpIssueConfirmOpen = value }"
    >
      <div class="confirm-sheet" :aria-busy="rkpIssueBusy">
        <p>{{ i18n.t('rkp_issue_warning') }}</p>
        <p>{{ i18n.t('rkp_protocol') }}: {{ i18n.t(`rkp_protocol_${rkpProtocol}`) }}</p>
        <p v-if="rkpIssueError" role="alert">{{ rkpIssueError }}</p>
        <p v-if="rkpIssueBusy" role="status">{{ i18n.t('rkp_issue_running') }}</p>
        <div class="confirm-actions">
          <MiuixButton :disabled="rkpIssueBusy" @click="rkpIssueConfirmOpen = false">
            {{ i18n.t('functional_button_cancel') }}
          </MiuixButton>
          <MiuixButton type="primary" :disabled="rkpIssueBusy" @click="issueRkpKeybox">
            <MiuixProgressIndicator v-if="rkpIssueBusy" type="circular" :size="18" />
            {{ i18n.t('rkp_issue_confirm') }}
          </MiuixButton>
        </div>
      </div>
    </MiuixDialog>
    <MiuixDialog
      :model-value="rkpImportConfirmOpen"
      :title="i18n.t('rkp_import_confirm_title')"
      :close-on-click-modal="!rkpIssueBusy"
      @update:model-value="value => { if (!rkpIssueBusy) rkpImportConfirmOpen = value }"
    >
      <div class="confirm-sheet" :aria-busy="rkpIssueBusy">
        <p>{{ i18n.t('rkp_import_warning') }}</p>
        <p v-if="pendingRkpImport">{{ formatRkpCreatedAt(pendingRkpImport.created_at) }}<br>{{ pendingRkpImport.chain_subjects.join(' · ') }}</p>
        <p v-if="rkpIssueError" role="alert">{{ rkpIssueError }}</p>
        <p v-if="rkpIssueBusy" role="status">{{ i18n.t('rkp_import_running') }}</p>
        <div class="confirm-actions">
          <MiuixButton :disabled="rkpIssueBusy" @click="rkpImportConfirmOpen = false">
            {{ i18n.t('functional_button_cancel') }}
          </MiuixButton>
          <MiuixButton type="primary" :disabled="rkpIssueBusy" @click="importRkpKeybox">
            <MiuixProgressIndicator v-if="rkpIssueBusy" type="circular" :size="18" />
            {{ i18n.t('rkp_import_confirm') }}
          </MiuixButton>
        </div>
      </div>
    </MiuixDialog>
    <SoterDialog ref="soterDialog" v-model="soterOpen" :cli="cli" @notify="notify" />
    <MiuixSnackbarHost />
  </div>
</template>
