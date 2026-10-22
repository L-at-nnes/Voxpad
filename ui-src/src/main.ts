import Vue from 'vue';
import App from './App.vue';
import store from './store';
import vuetify from './plugins/vuetify';
import './styles/voxpad.scss';
import i18n from './plugins/i18n';
import Toast from 'vue-toastification';
import 'vue-toastification/dist/index.css';
import $store from '@/store';
import { BoardViewMode, Output, PlayingSound, RecordingDevice, Settings, SortMode, Sound, Tab } from '@/types';
import vueDebounce from 'vue-debounce';
import { initTauriBridge } from '@/utils/tauri-bridge';

declare global {
  interface Window {
    // frontend callbacks the backend invokes
    hotkeyReceived: ((hotkey: string, hotkeyData: number[]) => void) | undefined;
    finishSound: ((playingSound: PlayingSound) => void) | undefined;
    onSoundPlayed: ((playingSound: PlayingSound) => void) | undefined;
    updateSound: ((playingSound: PlayingSound) => void) | undefined;
    onError: ((error: number) => void) | undefined;
    getTranslation: ((path: string) => string) | undefined;
    getStore: (() => typeof $store) | undefined;

    // backend functions
    isLinux: () => Promise<boolean | null>;
    getSettings: () => Promise<Settings>;
    changeSettings: (settings: Settings) => Promise<Settings>;
    getOutputs: () => Promise<Output[]>;

    getTabs: () => Promise<Tab[] | null>;
    createTab: (name: string, cover: string) => Promise<Tab | null>;
    setTabCover: (id: number, cover: string) => Promise<Tab | null>;
    setTabZoom: (id: number, zoom: number | null) => Promise<Tab | null>;
    setTabViewMode: (id: number, viewMode: BoardViewMode | null) => Promise<Tab | null>;
    pickImage: () => Promise<string | null>;
    importFolder: (id: number) => Promise<Tab | null>;
    refreshImportedFolders: (id: number) => Promise<Tab | null>;
    importFiles: (id: number) => Promise<Tab | null>;
    removeTab: (id: number) => Promise<Tab[] | null>;
    setSortMode: (tabId: number, sortMode: SortMode) => Promise<Tab | null>;

    playSound: (id: number, noRepeat?: boolean) => Promise<PlayingSound | null>;
    stopSounds: () => Promise<void>;
    stopSound: (id: number) => Promise<boolean | null>;
    pauseSound: (id: number) => Promise<PlayingSound | null>;
    resumeSound: (id: number) => Promise<PlayingSound | null>;
    seekSound: (id: number, seekTo: number) => Promise<PlayingSound | null>;
    deleteSound: (id: number) => Promise<boolean | null>;
    setCustomLocalVolume: (id: number, localVolume: number | null) => Promise<Sound | null>;
    setCustomRemoteVolume: (id: number, remoteVolume: number | null) => Promise<Sound | null>;
    setSoundRepeat: (id: number, loop: boolean | null) => Promise<Sound | null>;
    setSoundTags: (id: number, tags: string[]) => Promise<Sound | null>;
    createTag: (id: number, tag: string) => Promise<Tab | null>;
    reorderTags: (id: number, tags: string[]) => Promise<Tab | null>;
    renameTag: (id: number, oldTag: string, newTag: string) => Promise<Tab | null>;
    deleteTag: (id: number, tag: string) => Promise<Tab | null>;

    setHotkey: (id: number, hotkeys: number[]) => Promise<void>;
    getHotkeySequence: (hotkeys: number[]) => Promise<string | null>;
    requestHotkey: (state: boolean) => Promise<void>;

    setLaunchOnStartup: (enabled: boolean) => Promise<void>;
    isLaunchOnStartup: () => Promise<boolean | null>;

    // windows specific (VB-Cable)
    isVBCableProperlySetup: () => Promise<boolean | null>;
    getRecordingDevices: () => Promise<[RecordingDevice[], RecordingDevice] | null>;
    setupVBCable: (selectedRecording: string | null) => Promise<boolean | null>;
    installVBCable: () => Promise<boolean | null>;
    restartAsAdmin: () => Promise<void>;

    openUrl: (url: string) => Promise<void>;
  }
}

Vue.use(vueDebounce);
Vue.use(Toast, {
  position: 'bottom-right',
  timeout: 3000,
  closeOnClick: true,
  transition: 'Vue-Toastification__fade',
  maxToasts: 3,
  newestOnTop: true,
});

Vue.config.productionTip = false;

// Register the backend event listeners in parallel with mounting rather than blocking on it —
// if IPC registration is ever slow (or fails), the UI must still come up.
initTauriBridge().catch(err => console.error('initTauriBridge failed', err));

new Vue({
  store,
  vuetify,
  i18n,
  render: h => h(App),
}).$mount('#app');
