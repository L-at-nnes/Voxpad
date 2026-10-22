<template>
  <VxModal :value="open" :title="$t('settings.title')" :width="520" @input="close">
    <div class="vx-settings">
      <!-- Output device -->
      <section class="vx-set">
        <div class="vx-set__head">
          <h3>{{ $t('settings.outputDevice') }}</h3>
          <button class="vx-icon-btn" :title="$t('settings.refresh')" @click="refreshOutputs">
            <VxIcon name="repeat" :size="16" />
          </button>
        </div>
        <p class="vx-set__desc">{{ $t('settings.outputDesc') }}</p>
        <div class="vx-radio-list">
          <button
            v-for="o in outputs"
            :key="o.name"
            class="vx-radio"
            :class="{ 'vx-radio--on': isSelected(o.name) }"
            @click="selectOutput(o)"
          >
            <span class="vx-radio__dot"></span>
            <span class="vx-radio__label">{{ o.name }}</span>
          </button>
          <p v-if="outputs.length === 0" class="vx-set__desc">{{ $t('settings.noOutputs') }}</p>
        </div>
      </section>

      <!-- VB-Cable -->
      <section v-if="!isLinux" class="vx-set">
        <h3>{{ $t('settings.vbcable') }}</h3>
        <p class="vx-set__desc">{{ $t('settings.vbcableDesc') }}</p>
        <div class="vx-vbcable" :class="vbReady ? 'vx-vbcable--ok' : 'vx-vbcable--missing'">
          <span class="vx-vbcable__status">
            <VxIcon :name="vbReady ? 'check' : 'close'" :size="16" />
            {{ vbReady ? $t('settings.vbcableOk') : $t('settings.vbcableMissing') }}
          </span>
          <button v-if="!vbReady" class="vx-btn vx-btn--accent" :disabled="installing" @click="installVb">
            {{ installing ? $t('settings.vbcableInstalling') : $t('settings.vbcableInstall') }}
          </button>
        </div>
      </section>

      <!-- Behavior -->
      <section v-if="!isLinux" class="vx-set">
        <label class="vx-toggle">
          <span>
            <strong>{{ $t('settings.minimizeToTray') }}</strong>
            <small>{{ $t('settings.minimizeToTrayDesc') }}</small>
          </span>
          <input type="checkbox" :checked="settings.minimizeToTray" @change="toggleTray" />
          <span class="vx-toggle__track"></span>
        </label>
      </section>

      <!-- Startup -->
      <section v-if="!isLinux" class="vx-set">
        <label class="vx-toggle">
          <span>
            <strong>{{ $t('settings.startup') }}</strong>
            <small>{{ $t('settings.startupDesc') }}</small>
          </span>
          <input type="checkbox" :checked="settings.launchOnStartup" @change="toggleStartup" />
          <span class="vx-toggle__track"></span>
        </label>
      </section>

      <!-- Grid zoom default -->
      <section class="vx-set">
        <h3>{{ $t('settings.defaultZoom') }}</h3>
        <p class="vx-set__desc">{{ $t('settings.defaultZoomDesc') }}</p>
        <ZoomControl />
      </section>
    </div>
  </VxModal>
</template>

<script lang="ts">
import Vue from 'vue';
import { Output, Settings } from '@/types';
import VxModal from '@/components/VxModal.vue';
import VxIcon from '@/components/VxIcon.vue';
import ZoomControl from '@/components/ZoomControl.vue';

export default Vue.extend({
  name: 'SettingsDialog',
  components: { VxModal, VxIcon, ZoomControl },
  data: () => ({ vbReady: true, installing: false }),
  computed: {
    open(): boolean {
      return this.$store.getters.settingsDialog;
    },
    settings(): Settings {
      return this.$store.getters.settings;
    },
    outputs(): Output[] {
      return this.$store.getters.outputs;
    },
    isLinux(): boolean {
      return !!this.$store.getters.isLinux;
    },
  },
  watch: {
    open(v: boolean) {
      if (v) {
        this.refreshOutputs();
        this.checkVbCable();
      }
    },
  },
  methods: {
    close() {
      this.$store.commit('setSettingsDialog', false);
    },
    isSelected(name: string): boolean {
      return this.settings.outputs.includes(name);
    },
    selectOutput(o: Output) {
      this.$store.dispatch('setSelectedOutputs', [o]);
    },
    refreshOutputs() {
      this.$store.dispatch('getOutputs');
    },
    toggleStartup(e: Event) {
      this.$store.dispatch('setLaunchOnStartup', (e.target as HTMLInputElement).checked);
    },
    toggleTray(e: Event) {
      this.$store.dispatch('setMinimizeToTray', (e.target as HTMLInputElement).checked);
    },
    async checkVbCable() {
      if (this.isLinux) return;
      this.vbReady = (await window.isVBCableProperlySetup()) || false;
    },
    async installVb() {
      this.installing = true;
      try {
        await window.installVBCable();
      } finally {
        this.installing = false;
        await this.checkVbCable();
      }
    },
  },
});
</script>

<style scoped lang="scss">
.vx-settings {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.vx-set {
  &__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  h3 {
    margin: 0 0 4px;
    font-size: 14px;
    font-weight: 700;
  }
  &__desc {
    margin: 0 0 12px;
    font-size: 12px;
    color: var(--text-dim);
  }
}

.vx-radio-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.vx-radio {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 11px 14px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text);
  font-size: 13px;
  text-align: left;
  transition: border-color 0.14s var(--ease), background 0.14s var(--ease);

  &__dot {
    flex: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 2px solid var(--text-dim);
    transition: all 0.14s var(--ease);
  }
  &__label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  &:hover {
    background: var(--surface-3);
  }
  &--on {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  &--on &__dot {
    border-color: var(--accent);
    background: var(--accent);
    box-shadow: inset 0 0 0 3px var(--surface);
  }
}

.vx-vbcable {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);

  &__status {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    font-weight: 500;
  }
  &--ok &__status {
    color: oklch(0.78 0.15 150);
  }
  &--missing &__status {
    color: var(--danger);
  }
}

.vx-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  cursor: pointer;

  span:first-child {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  strong {
    font-size: 14px;
    font-weight: 600;
  }
  small {
    font-size: 12px;
    color: var(--text-dim);
  }
  input {
    display: none;
  }
  &__track {
    position: relative;
    width: 40px;
    height: 22px;
    border-radius: 99px;
    background: var(--surface-3);
    transition: background 0.16s var(--ease);
    flex: none;
  }
  &__track::after {
    content: '';
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text);
    transition: transform 0.16s var(--ease);
  }
  input:checked + &__track {
    background: var(--accent);
  }
  input:checked + &__track::after {
    transform: translateX(18px);
  }
}
</style>
