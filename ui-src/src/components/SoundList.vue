<template>
  <section class="vx-list-wrap" @click="deselect" @contextmenu.prevent="deselect">
    <div v-if="filtered.length" class="vx-list">
      <div class="vx-list__head">
        <span class="vx-list__col vx-list__col--name">{{ $t('list.name') }}</span>
        <span class="vx-list__col vx-list__col--album">{{ $t('list.album') }}</span>
        <span class="vx-list__col vx-list__col--duration">{{ $t('list.duration') }}</span>
      </div>
      <div
        v-for="sound in filtered"
        :key="sound.id"
        class="vx-list__row"
        :class="{ 'vx-list__row--playing': isPlaying(sound), 'vx-list__row--selected': isSelected(sound) }"
        @mousedown="pressSound($event, sound, dropOnTag)"
        @click.stop="rowClick(sound)"
        @contextmenu.prevent.stop="onRightClick(sound, $event)"
      >
        <span class="vx-list__play">
          <VxIcon :name="isPlaying(sound) ? 'stop' : 'play'" :size="13" filled />
        </span>
        <span class="vx-list__col vx-list__col--name" :title="sound.name">{{ sound.name }}</span>
        <span class="vx-list__col vx-list__col--album">{{ sound.album || '—' }}</span>
        <span class="vx-list__col vx-list__col--duration">
          {{ sound.lengthInMs != null ? fmtMs(sound.lengthInMs) : '—' }}
        </span>
      </div>
    </div>

    <div v-else class="vx-list-empty">
      <template v-if="tab.sounds.length === 0">
        <h3>{{ $t('empty.noSoundsTitle') }}</h3>
        <p>{{ $t('empty.noSoundsBody') }}</p>
        <div class="vx-list-empty__actions">
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
      <div v-if="ctx" class="vx-ctx" :style="ctxStyle" @click.stop @contextmenu.prevent.stop>
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
import { PlayingSound, Sound, Tab } from '@/types';
import VxIcon from '@/components/VxIcon.vue';
import { pressSound } from '@/utils/soundDrag';

interface CtxState {
  sound: Sound;
  x: number;
  y: number;
}

function fmtMs(ms: number): string {
  const totalSec = Math.floor(ms / 1000);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  return `${m}:${String(s).padStart(2, '0')}`;
}

export default Vue.extend({
  name: 'SoundList',
  components: { VxIcon },
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
    current(): PlayingSound | null {
      return this.$store.getters.currentSound;
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
    pressSound,
    dropOnTag(sound: Sound, tag: string) {
      this.$store.dispatch('addSoundTag', { sound, tag });
    },
    fmtMs,
    isPlaying(sound: Sound): boolean {
      return !!this.current && this.current.sound.id === sound.id;
    },
    isSelected(sound: Sound): boolean {
      const sel = this.$store.getters.selectedSound as Sound | null;
      return !!sel && sel.id === sound.id;
    },
    rowClick(sound: Sound) {
      this.ctx = null;
      if (this.isPlaying(sound)) {
        this.$store.dispatch('stopSounds');
      } else {
        this.$store.dispatch('playSound', sound);
      }
      this.$store.dispatch('selectSound', sound);
    },
    deselect() {
      this.ctx = null;
      this.$store.dispatch('selectSound', null);
    },
    onRightClick(sound: Sound, e: MouseEvent) {
      const MENU_W = 180, MENU_H = 120;
      const x = Math.min(e.clientX, window.innerWidth - MENU_W - 8);
      const y = Math.min(e.clientY, window.innerHeight - MENU_H - 8);
      this.ctx = { sound, x, y };
      this.$store.dispatch('selectSound', sound);
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
.vx-list-wrap {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 22px 20px 28px;
  position: relative;
}

.vx-list {
  display: flex;
  flex-direction: column;

  &__head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 12px 8px 34px;
    color: var(--text-dim);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  &__row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 12px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background 0.12s var(--ease);

    &:hover {
      background: var(--surface-2);
    }
    &--playing {
      background: var(--accent-soft);
    }
    &--selected &__col--name {
      color: var(--accent);
    }
  }

  &__play {
    display: grid;
    place-items: center;
    flex: none;
    width: 18px;
    color: var(--text-dim);
  }
  &__row--playing &__play {
    color: var(--accent);
  }

  &__col {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;

    &--name {
      flex: 1;
      color: var(--text);
      font-weight: 500;
    }
    &--album {
      flex: 0 0 220px;
      color: var(--text-dim);
    }
    &--duration {
      flex: 0 0 64px;
      text-align: right;
      color: var(--text-dim);
      font-variant-numeric: tabular-nums;
    }
  }
}

.vx-list-empty {
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
