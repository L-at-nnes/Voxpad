<template>
  <section class="vx-grid-wrap" @click="deselect" @contextmenu.prevent="deselect">
    <div v-if="filtered.length" class="vx-grid" :style="gridStyle">
      <SoundTile
        v-for="sound in filtered"
        :key="sound.id"
        :sound="sound"
        @play="play(sound)"
        @stop="$store.dispatch('stopSounds')"

        @rightclick="onRightClick(sound, $event)"
        @keybind="captureHotkey(sound)"
      />
    </div>

    <div v-else class="vx-grid-empty">
      <template v-if="tab.sounds.length === 0">
        <h3>{{ $t('empty.noSoundsTitle') }}</h3>
        <p>{{ $t('empty.noSoundsBody') }}</p>
        <div class="vx-grid-empty__actions">
          <button class="vx-btn vx-btn--accent" @click.stop="$store.dispatch('importFiles')">
            <VxIcon name="file" :size="17" /> {{ $t('board.importFiles') }}
          </button>
          <button class="vx-btn" @click.stop="$store.dispatch('importFolder')">
            <VxIcon name="folder" :size="17" /> {{ $t('board.importFolder') }}
          </button>
        </div>
      </template>
      <template v-else>
        <h3>{{ $t('empty.noMatchTitle') }}</h3>
        <p>{{ $t('empty.noMatchBody') }}</p>
      </template>
    </div>

    <!-- Context menu -->
    <transition name="vx-ctx-fade">
      <div
        v-if="ctx"
        class="vx-ctx"
        :style="ctxStyle"
        @click.stop
        @contextmenu.prevent.stop
      >
        <button class="vx-ctx__item" @click="ctxQueue">
          <VxIcon name="repeat" :size="14" />
          {{ $t('grid.addToQueue') }}
        </button>
        <button class="vx-ctx__item" @click="ctxTags">
          <VxIcon name="tag" :size="14" />
          {{ $t('tags.manage') }}
        </button>
      </div>
    </transition>
  </section>
</template>

<script lang="ts">
import Vue, { PropType } from 'vue';
import { Sound, Tab } from '@/types';
import SoundTile from '@/components/SoundTile.vue';
import VxIcon from '@/components/VxIcon.vue';

const TILE_SIZES = [76, 96, 120, 150, 182, 220];

interface CtxState {
  sound: Sound;
  x: number;
  y: number;
}

export default Vue.extend({
  name: 'SoundGrid',
  components: { SoundTile, VxIcon },
  props: {
    tab: { type: Object as PropType<Tab>, required: true },
    query: { type: String, default: '' },
  },
  data: () => ({ ctx: null as CtxState | null }),
  computed: {
    filtered(): Sound[] {
      const q = this.query.trim().toLowerCase();
      if (!q) return this.tab.sounds;
      return this.tab.sounds.filter(s => s.name.toLowerCase().includes(q));
    },
    gridStyle(): Record<string, string> {
      const size = TILE_SIZES[this.$store.getters.effectiveZoom] || TILE_SIZES[2];
      return { gridTemplateColumns: `repeat(auto-fill, minmax(${size}px, 1fr))` };
    },
    ctxStyle(): Record<string, string> {
      if (!this.ctx) return {};
      return { left: `${this.ctx.x}px`, top: `${this.ctx.y}px` };
    },
  },
  mounted() {
    document.addEventListener('click', this.closeCtx);
    document.addEventListener('contextmenu', this.closeCtx);
  },
  beforeDestroy() {
    document.removeEventListener('click', this.closeCtx);
    document.removeEventListener('contextmenu', this.closeCtx);
  },
  methods: {
    play(sound: Sound) {
      this.ctx = null;
      this.$store.dispatch('playSound', sound);
      this.$store.dispatch('selectSound', sound);
    },
    select(sound: Sound) {
      this.$store.dispatch('selectSound', sound);
    },
    deselect() {
      this.ctx = null;
      this.$store.dispatch('selectSound', null);
    },
    captureHotkey(sound: Sound) {
      this.$store.dispatch('selectSound', sound);
      this.$store.commit('setCapturingHotkeyFor', sound.id);
      window.requestHotkey(true);
    },
    onRightClick(sound: Sound, e: MouseEvent) {
      // Position the menu so it stays inside the viewport
      const MENU_W = 180, MENU_H = 120;
      const x = Math.min(e.clientX, window.innerWidth - MENU_W - 8);
      const y = Math.min(e.clientY, window.innerHeight - MENU_H - 8);
      this.ctx = { sound, x, y };
      this.select(sound);
    },
    closeCtx() {
      this.ctx = null;
    },
    ctxQueue() {
      if (this.ctx) this.$store.dispatch('addToQueue', this.ctx.sound);
      this.ctx = null;
    },
    ctxTags() {
      if (this.ctx) this.$store.commit('setTagDialogSound', this.ctx.sound.id);
      this.ctx = null;
    },
  },
});
</script>

<style scoped lang="scss">
.vx-grid-wrap {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 22px 20px 28px;
  position: relative;
}

.vx-grid {
  display: grid;
  gap: 16px;
}

.vx-grid-empty {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  gap: 8px;
  color: var(--text-dim);

  h3 {
    margin: 0;
    color: var(--text);
    font-size: 18px;
  }
  p {
    margin: 0 0 10px;
    max-width: 320px;
  }
  &__actions {
    display: flex;
    gap: 10px;
  }
}

.vx-ctx {
  position: fixed;
  z-index: 9999;
  min-width: 170px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  box-shadow: 0 8px 24px oklch(0 0 0 / 0.35);
  padding: 4px;
  display: flex;
  flex-direction: column;

  &__item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 12px;
    border: none;
    background: none;
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    border-radius: var(--radius-sm);
    cursor: pointer;
    text-align: left;
    transition: background 0.12s var(--ease);

    &:hover {
      background: var(--surface-3);
    }
  }
}

.vx-ctx-fade-enter-active,
.vx-ctx-fade-leave-active {
  transition: opacity 0.1s var(--ease), transform 0.1s var(--ease);
}
.vx-ctx-fade-enter,
.vx-ctx-fade-leave-to {
  opacity: 0;
  transform: scale(0.96) translateY(-4px);
}
</style>
