<template>
  <section class="vx-album-wrap">
    <!-- Album grid -->
    <div v-if="!openAlbum" class="vx-album-grid-outer">
      <div v-if="albums.length" class="vx-album-grid">
        <div
          v-for="album in albums"
          :key="album.name"
          class="vx-album-card"
          @click="openAlbum = album.name"
        >
          <div class="vx-album-card__art" :style="{ background: gradientFromName(album.name) }">
            <VxIcon name="disc" :size="30" />
          </div>
          <div class="vx-album-card__name" :title="album.name">{{ album.name }}</div>
          <div class="vx-album-card__count">{{ $tc('album.trackCount', album.sounds.length) }}</div>
        </div>
      </div>
      <div v-else class="vx-album-empty">
        <h3>{{ $t('album.noAlbumsTitle') }}</h3>
        <p>{{ $t('album.noAlbumsBody') }}</p>
      </div>
    </div>

    <!-- Album detail -->
    <div v-else class="vx-album-detail">
      <div class="vx-album-detail__head">
        <button class="vx-album-back" :title="$t('album.back')" @click="openAlbum = null">
          <VxIcon name="arrow-left" :size="17" />
        </button>
        <div class="vx-album-detail__art" :style="{ background: gradientFromName(openAlbum) }">
          <VxIcon name="disc" :size="20" />
        </div>
        <div class="vx-album-detail__info">
          <h2>{{ openAlbum }}</h2>
          <span>{{ $tc('album.trackCount', currentAlbumSounds.length) }}</span>
        </div>
        <div class="vx-album-detail__actions">
          <button class="vx-btn vx-btn--accent" @click="playAlbum(true)">
            <VxIcon name="play" :size="15" filled /> {{ $t('album.playSequential') }}
          </button>
          <button class="vx-btn" @click="playAlbum(false)">
            <VxIcon name="shuffle" :size="15" /> {{ $t('album.playShuffled') }}
          </button>
        </div>
      </div>

      <div v-if="trackView === 'list'" class="vx-list">
        <div
          v-for="sound in currentAlbumSounds"
          :key="sound.id"
          class="vx-list__row"
          :class="{ 'vx-list__row--playing': isPlaying(sound) }"
          @mousedown="pressSound($event, sound, dropOnTag)"
        @click="rowClick(sound)"
        >
          <span class="vx-list__play">
            <VxIcon :name="isPlaying(sound) ? 'stop' : 'play'" :size="13" filled />
          </span>
          <span class="vx-list__col vx-list__col--name" :title="sound.name">{{ sound.name }}</span>
          <span class="vx-list__col vx-list__col--duration">
            {{ sound.lengthInMs != null ? fmtMs(sound.lengthInMs) : '—' }}
          </span>
        </div>
      </div>

      <div v-else class="vx-album-tracks-grid">
        <SoundTile
          v-for="sound in currentAlbumSounds"
          :key="sound.id"
          :sound="sound"
          @play="rowClick(sound)"
          @stop="rowClick(sound)"
          @keybind="captureHotkey(sound)"
        />
      </div>

      <div class="vx-album-detail__viewmode">
        <div class="vx-viewmode">
          <button
            v-for="opt in trackViewOptions"
            :key="opt.mode"
            class="vx-viewmode__btn"
            :class="{ 'vx-viewmode__btn--active': trackView === opt.mode }"
            :title="$t(opt.label)"
            @click="setTrackView(opt.mode)"
          >
            <VxIcon :name="opt.icon" :size="16" />
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<script lang="ts">
import Vue, { PropType } from 'vue';
import { PlayingSound, Sound, Tab } from '@/types';
import VxIcon from '@/components/VxIcon.vue';
import { pressSound } from '@/utils/soundDrag';
import SoundTile from '@/components/SoundTile.vue';
import { gradientFromName } from '@/utils/visual';

type TrackView = 'list' | 'grid';
const TRACK_VIEW_KEY = 'voxpad.albumTrackView';

function loadTrackView(): TrackView {
  try {
    return localStorage.getItem(TRACK_VIEW_KEY) === 'grid' ? 'grid' : 'list';
  } catch {
    return 'list';
  }
}

interface AlbumGroup {
  name: string;
  sounds: Sound[];
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
  name: 'AlbumView',
  components: { VxIcon, SoundTile },
  props: {
    tab: { type: Object as PropType<Tab>, required: true },
    query: { type: String, default: '' },
  },
  data: () => ({
    openAlbum: null as string | null,
    trackView: loadTrackView(),
    trackViewOptions: [
      { mode: 'list', icon: 'list-view', label: 'album.viewList' },
      { mode: 'grid', icon: 'grid-view', label: 'album.viewGrid' },
    ] as { mode: TrackView; icon: string; label: string }[],
  }),
  computed: {
    albums(): AlbumGroup[] {
      const groups = new Map<string, Sound[]>();
      for (const sound of this.tab.sounds) {
        const album = (sound.album || '').trim();
        if (!album) continue;
        const list = groups.get(album) || [];
        list.push(sound);
        groups.set(album, list);
      }
      const q = this.query.trim().toLowerCase();
      return Array.from(groups.entries())
        .map(([name, sounds]) => ({ name, sounds }))
        .filter(a => !q || a.name.toLowerCase().includes(q))
        .sort((a, b) => a.name.localeCompare(b.name));
    },
    currentAlbumSounds(): Sound[] {
      if (!this.openAlbum) return [];
      const group = this.albums.find(a => a.name === this.openAlbum)
        || { name: this.openAlbum, sounds: this.tab.sounds.filter(s => (s.album || '').trim() === this.openAlbum) };
      return [...group.sounds].sort((a, b) => {
        const ta = parseInt(a.trackNumber || '', 10);
        const tb = parseInt(b.trackNumber || '', 10);
        if (!isNaN(ta) && !isNaN(tb) && ta !== tb) return ta - tb;
        if (!isNaN(ta) !== !isNaN(tb)) return !isNaN(ta) ? -1 : 1;
        return a.name.localeCompare(b.name);
      });
    },
    current(): PlayingSound | null {
      return this.$store.getters.currentSound;
    },
  },
  watch: {
    tab() {
      this.openAlbum = null;
    },
  },
  methods: {
    pressSound,
    dropOnTag(sound: Sound, tag: string) {
      this.$store.dispatch('addSoundTag', { sound, tag });
    },
    fmtMs,
    gradientFromName,
    isPlaying(sound: Sound): boolean {
      return !!this.current && this.current.sound.id === sound.id;
    },
    rowClick(sound: Sound) {
      if (this.isPlaying(sound)) {
        this.$store.dispatch('stopSounds');
      } else {
        this.$store.dispatch('playSound', sound);
      }
    },
    setTrackView(mode: TrackView) {
      this.trackView = mode;
      try {
        localStorage.setItem(TRACK_VIEW_KEY, mode);
      } catch {
        // preference just won't persist
      }
    },
    captureHotkey(sound: Sound) {
      this.$store.dispatch('selectSound', sound);
      this.$store.commit('setCapturingHotkeyFor', sound.id);
      window.requestHotkey(true);
    },
    playAlbum(sequential: boolean) {
      this.$store.dispatch('playAlbumTracks', { tab: this.tab, sounds: this.currentAlbumSounds, sequential });
    },
  },
});
</script>

<style scoped lang="scss">
.vx-album-wrap {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 22px 20px 28px;
  position: relative;
}

.vx-album-grid-outer {
  height: 100%;
}

.vx-album-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 18px;
}

.vx-album-card {
  display: flex;
  flex-direction: column;
  gap: 7px;
  cursor: pointer;

  &__art {
    aspect-ratio: 1 / 1;
    border-radius: var(--radius);
    display: grid;
    place-items: center;
    color: oklch(1 0 0 / 0.55);
    transition: transform 0.14s var(--ease), box-shadow 0.18s var(--ease);
  }
  &:hover &__art {
    transform: translateY(-3px);
    box-shadow: 0 10px 24px oklch(0 0 0 / 0.35);
  }

  &__name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding: 0 2px;
  }

  &__count {
    font-size: 11px;
    color: var(--text-dim);
    text-align: center;
  }
}

.vx-album-empty {
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
    margin: 0;
    max-width: 320px;
  }
}

.vx-album-tracks-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 18px;
}

.vx-viewmode {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 3px;
  height: 38px;
  border-radius: 99px;
  background: var(--surface);
  border: 1px solid var(--border);
  box-shadow: 0 4px 14px oklch(0 0 0 / 0.35);

  &__btn {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 99px;
    border: none;
    background: transparent;
    color: var(--text-dim);
    cursor: pointer;
    transition: background 0.14s var(--ease), color 0.14s var(--ease);

    &:hover {
      color: var(--text);
    }
    &--active {
      background: var(--accent-soft);
      color: var(--accent);
    }
  }
}

.vx-album-detail {
  min-height: 100%;
  display: flex;
  flex-direction: column;

  // pinned bottom-right of the scroll area
  &__viewmode {
    position: sticky;
    bottom: 0;
    margin-top: auto;
    padding-top: 16px;
    align-self: flex-end;
  }

  &__head {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 20px;
  }

  &__art {
    flex: none;
    width: 52px;
    height: 52px;
    border-radius: var(--radius);
    display: grid;
    place-items: center;
    color: oklch(1 0 0 / 0.55);
  }

  &__info {
    flex: 1;
    min-width: 0;

    h2 {
      margin: 0;
      font-size: 18px;
      color: var(--text);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    span {
      font-size: 12px;
      color: var(--text-dim);
    }
  }

  &__actions {
    display: flex;
    gap: 10px;
    flex: none;
  }
}

.vx-album-back {
  display: grid;
  place-items: center;
  flex: none;
  width: 34px;
  height: 34px;
  border-radius: 50%;
  border: 1px solid var(--border);
  background: var(--surface);
  color: var(--text-dim);
  cursor: pointer;
  transition: background 0.12s var(--ease), color 0.12s var(--ease);

  &:hover {
    color: var(--text);
    background: var(--surface-2);
  }
}

.vx-list {
  display: flex;
  flex-direction: column;

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
    &--duration {
      flex: 0 0 64px;
      text-align: right;
      color: var(--text-dim);
      font-variant-numeric: tabular-nums;
    }
  }
}
</style>
