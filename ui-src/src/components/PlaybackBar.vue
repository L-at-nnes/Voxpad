<template>
  <transition name="vx-bar-slide">
    <div v-if="show" class="vx-playbar">
      <!-- Left: cover + name -->
      <div class="vx-playbar__info">
        <div class="vx-playbar__art" :style="{ background: artGradient }">
          <VxIcon name="music" :size="16" />
        </div>
        <span class="vx-playbar__name" :title="playing.sound.name">{{ playing.sound.name }}</span>
      </div>

      <!-- Center: controls + progress -->
      <div class="vx-playbar__center">
        <div class="vx-playbar__controls">
          <!-- Previous track -->
          <button class="vx-playbar__skip" :class="{ 'vx-playbar__skip--disabled': !hasPrev }" :title="$t('playbar.prevTrack')" @click="skipPrev">
            <VxIcon name="skip-prev" :size="18" />
          </button>

          <!-- -10s -->
          <button class="vx-playbar__skip" :title="$t('playbar.rewind')" @click="seek(-10000)">
            <VxIcon name="rewind" :size="18" />
          </button>

          <!-- Pause / Play -->
          <button class="vx-playbar__play" :title="paused ? $t('playbar.play') : $t('playbar.pause')" @click="togglePause">
            <VxIcon :name="paused ? 'play' : 'pause'" :size="20" filled />
          </button>

          <!-- +10s -->
          <button class="vx-playbar__skip" :title="$t('playbar.forward')" @click="seek(10000)">
            <VxIcon name="forward" :size="18" />
          </button>

          <!-- Next track -->
          <button class="vx-playbar__skip" :class="{ 'vx-playbar__skip--disabled': !hasNext }" :title="$t('playbar.nextTrack')" @click="skipNext">
            <VxIcon name="skip-next" :size="18" />
          </button>
        </div>

        <div class="vx-playbar__progress">
          <span class="vx-playbar__time">{{ elapsedFmt }}</span>
          <div
            v-if="!unknownDuration"
            class="vx-playbar__bar"
            @click="seekToRatio($event)"
            @mousedown.prevent
          >
            <div class="vx-playbar__fill" :style="{ width: progress + '%' }"></div>
            <div class="vx-playbar__thumb" :style="{ left: progress + '%' }"></div>
          </div>
          <div v-else class="vx-playbar__bar vx-playbar__bar--unknown">
            <div class="vx-playbar__fill vx-playbar__fill--pulse"></div>
          </div>
          <span class="vx-playbar__time vx-playbar__time--rem">
            {{ unknownDuration ? '?' : '-' + remainingFmt }}
          </span>
        </div>
      </div>

      <!-- Right: file metadata -->
      <div class="vx-playbar__meta">
        <span class="vx-playbar__meta-artist">{{ meta.artist }}</span>
        <span class="vx-playbar__meta-album">{{ meta.album }}</span>
        <span v-if="meta.extra" class="vx-playbar__meta-extra">{{ meta.extra }}</span>
      </div>
    </div>
  </transition>
</template>

<script lang="ts">
import Vue from 'vue';
import { PlayingSound } from '@/types';
import { tileGradientFromName } from '@/utils/visual';
import VxIcon from '@/components/VxIcon.vue';

const MIN_DURATION_MS = 60000; // 1 minute

function fmtMs(ms: number): string {
  if (ms < 1000) return `0:0${Math.floor(ms / 100)}`;
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(sec).padStart(2, '0')}`;
  return `${m}:${String(sec).padStart(2, '0')}`;
}

export default Vue.extend({
  name: 'PlaybackBar',
  components: { VxIcon },
  computed: {
    playing(): PlayingSound | null {
      return this.$store.getters.currentSound;
    },
    durationMs(): number {
      if (!this.playing) return 0;
      return this.playing.lengthInMs || this.playing.sound.lengthInMs || 0;
    },
    unknownDuration(): boolean {
      return this.durationMs === 0;
    },
    show(): boolean {
      if (!this.playing) return false;
      // Show for sounds >= 1 min, when duration is unknown (VBR MP3 / old imports), or whenever
      // shuffle is cycling through short soundboard clips — those still need working
      // prev/next/seek controls, which only live in this bar (the compact Inspector "now
      // playing" card only has a stop button).
      return this.durationMs >= MIN_DURATION_MS || this.unknownDuration || this.shuffleActive;
    },
    paused(): boolean {
      return !!this.playing?.paused;
    },
    progress(): number {
      return this.$store.getters.displayProgress * 100;
    },
    artGradient(): string {
      return this.playing ? tileGradientFromName(this.playing.sound.name) : '';
    },
    hasNext(): boolean {
      return this.$store.getters.canSkipNext;
    },
    shuffleActive(): boolean {
      return this.$store.getters.shuffleActive;
    },
    hasPrev(): boolean {
      return this.$store.getters.playHistory.length > 0;
    },
    meta(): { artist: string; album: string; extra: string } {
      const s = this.playing?.sound;
      const tag = (v: string | null | undefined) => (v == null || v === '' ? '—' : v);
      const artist = tag(s?.artist);
      const album = tag(s?.album);
      const parts: string[] = [];
      if (s?.year) parts.push(s.year);
      if (s?.trackNumber) parts.push(`#${s.trackNumber}`);
      if (s?.discNumber) parts.push(`Disc ${s.discNumber}`);
      return { artist, album, extra: parts.join(' · ') };
    },
    rawElapsedMs(): number {
      const tab = this.$store.getters.currentTab;
      if (!tab) return 0;
      const ps = this.$store.state.tabPlaying[tab.id];
      if (!ps) return 0;
      return ps.sound.paused
        ? ps.progBaseMs
        : ps.progBaseMs + (performance.now() - ps.progBaseT);
    },
    elapsedFmt(): string {
      if (!this.playing) return '0:00';
      if (this.unknownDuration) return fmtMs(this.rawElapsedMs);
      return fmtMs(this.$store.getters.displayProgress * this.durationMs);
    },
    remainingFmt(): string {
      if (!this.playing || this.unknownDuration) return '0:00';
      return fmtMs(Math.max(0, (1 - this.$store.getters.displayProgress) * this.durationMs));
    },
  },
  methods: {
    togglePause() {
      if (this.paused) {
        this.$store.dispatch('resumeActiveTabSound');
      } else {
        this.$store.dispatch('pauseActiveTabSound');
      }
    },
    seek(deltaMs: number) {
      this.$store.dispatch('seekActiveTabSound', deltaMs);
    },
    skipNext() {
      this.$store.dispatch('skipToNext');
    },
    skipPrev() {
      this.$store.dispatch('skipToPrevious');
    },
    seekToRatio(e: MouseEvent) {
      const bar = e.currentTarget as HTMLElement;
      const rect = bar.getBoundingClientRect();
      const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      this.$store.dispatch('seekActiveTabToPosition', ratio);
    },
  },
});
</script>

<style scoped lang="scss">
.vx-playbar {
  flex: none;
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  align-items: center;
  gap: 20px;
  padding: 0 20px;
  height: 72px;
  background: var(--surface);
  border-top: 1px solid var(--border);
}

.vx-playbar__info {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}
.vx-playbar__art {
  width: 38px;
  height: 38px;
  border-radius: var(--radius-sm);
  display: grid;
  place-items: center;
  color: oklch(1 0 0 / 0.6);
  flex: none;
}
.vx-playbar__name {
  font-size: 13px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  color: var(--text);
}

.vx-playbar__center {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  min-width: 320px;
}

.vx-playbar__controls {
  display: flex;
  align-items: center;
  gap: 10px;
}
.vx-playbar__skip {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  border: none;
  background: none;
  color: var(--text-dim);
  border-radius: 50%;
  cursor: pointer;
  transition: color 0.12s var(--ease), background 0.12s var(--ease);

  &:hover {
    color: var(--text);
    background: var(--surface-2);
  }

  &--disabled {
    opacity: 0.3;
    pointer-events: none;
  }
}
.vx-playbar__play {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  border-radius: 50%;
  border: none;
  background: var(--accent);
  color: oklch(1 0 0 / 0.95);
  cursor: pointer;
  transition: transform 0.12s var(--ease), background 0.12s var(--ease);

  &:hover {
    transform: scale(1.06);
    background: oklch(from var(--accent) l c h / 0.9);
  }
  &:active {
    transform: scale(0.96);
  }
}

.vx-playbar__progress {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
}
.vx-playbar__time {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-dim);
  white-space: nowrap;
  min-width: 36px;

  &--rem {
    text-align: right;
    color: var(--accent);
  }
}
.vx-playbar__bar {
  flex: 1;
  height: 4px;
  background: var(--surface-3);
  border-radius: 99px;
  cursor: pointer;
  position: relative;
  transition: height 0.1s var(--ease);

  &:hover {
    height: 6px;

    .vx-playbar__thumb {
      opacity: 1;
    }
  }
}
.vx-playbar__fill {
  position: absolute;
  left: 0;
  top: 0;
  height: 100%;
  border-radius: 99px;
  background: var(--accent);
  pointer-events: none;
}
.vx-playbar__thumb {
  position: absolute;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--accent);
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.1s var(--ease);
}

.vx-playbar__bar--unknown {
  cursor: default;
}
.vx-playbar__fill--pulse {
  width: 30% !important;
  animation: vx-bar-pulse 1.6s ease-in-out infinite;
}
@keyframes vx-bar-pulse {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 1; }
}

.vx-playbar__meta {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
  min-width: 0;
  overflow: hidden;
}
.vx-playbar__meta-artist {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
}
.vx-playbar__meta-album {
  font-size: 12px;
  color: var(--text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
}
.vx-playbar__meta-extra {
  font-size: 11px;
  color: oklch(from var(--text-dim) l c h / 0.6);
  white-space: nowrap;
}

/* Slide in from bottom */
.vx-bar-slide-enter-active,
.vx-bar-slide-leave-active {
  transition: height 0.2s var(--ease), opacity 0.2s var(--ease);
  overflow: hidden;
}
.vx-bar-slide-enter,
.vx-bar-slide-leave-to {
  height: 0 !important;
  opacity: 0;
}
</style>
