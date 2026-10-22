<template>
  <aside class="vx-inspector">
    <transition name="vx-face" mode="out-in">
      <!-- Selected sound -->
      <div v-if="sound" key="sound" class="vx-face">
        <div class="vx-face__cover" :style="{ background: soundGradient }">
          <VxIcon name="music" :size="34" />
        </div>
        <div class="vx-face__title-row">
          <h2 class="vx-face__title" :title="sound.name">{{ sound.name }}</h2>
          <span v-if="sound.lengthInMs != null" class="vx-face__dur">{{ fmtMs(sound.lengthInMs) }}</span>
        </div>

        <!-- Elapsed / remaining while this sound is playing -->
        <transition name="vx-collapse">
          <div v-if="isThisPlaying && playing" class="vx-progress-row">
            <span class="vx-progress-row__elapsed">{{ elapsedFmt }}</span>
            <div class="vx-progress-row__bar">
              <span :style="{ width: progress + '%' }"></span>
            </div>
            <span class="vx-progress-row__remaining">-{{ remainingFmt }}</span>
          </div>
        </transition>

        <div class="vx-keybind-row" :class="{ 'vx-keybind-row--set': capturing || sound.hotkeySequence }">
          <button class="vx-keybind-row__main" @click="capture">
            <VxIcon name="keyboard" :size="16" />
            <span>{{ keyText }}</span>
          </button>
          <button
            v-if="sound.hotkeySequence && !capturing"
            class="vx-keybind-row__clear"
            :title="$t('sound.removeKeybind')"
            @click="clearKey"
          >
            <VxIcon name="close" :size="14" />
          </button>
        </div>

        <div class="vx-field">
          <div class="vx-field__head">
            <span><VxIcon name="volume" :size="15" /> {{ $t('inspector.volume') }}</span>
            <button v-if="hasCustomVolume" class="vx-link" @click="resetVolume">
              {{ $t('inspector.default') }}
            </button>
          </div>
          <VxSlider :value="volume" :min="0" :max="100" @input="setVolume" />
        </div>

        <div class="vx-field">
          <div class="vx-field__head">
            <span><VxIcon name="repeat" :size="15" /> {{ $t('inspector.loop') }}</span>
            <button v-if="loopOverridden" class="vx-link" @click="resetLoop">
              {{ $t('inspector.default') }}
            </button>
          </div>
          <label class="vx-switch">
            <input type="checkbox" :checked="effectiveLoop" @change="toggleLoop" />
            <span class="vx-switch__track"></span>
            <span class="vx-switch__label">{{ effectiveLoop ? $t('common.on') : $t('common.off') }}</span>
          </label>
        </div>

        <div class="vx-field">
          <div class="vx-field__head">
            <span><VxIcon name="tag" :size="15" /> {{ $t('tags.label') }}</span>
          </div>
          <div class="vx-taged">
            <span v-for="tag in sound.tags" :key="tag" class="vx-taged__chip">
              {{ tag }}
              <button :title="$t('tags.remove')" @click="removeTag(tag)">
                <VxIcon name="close" :size="11" />
              </button>
            </span>
            <input
              v-model="tagDraft"
              class="vx-taged__input"
              list="vx-tag-suggestions"
              :placeholder="$t('tags.add')"
              maxlength="32"
              spellcheck="false"
              @keydown.enter.prevent="addTag"
              @keydown.188.prevent="addTag"
              @change="addTag"
            />
            <datalist id="vx-tag-suggestions">
              <option v-for="t in suggestions" :key="t" :value="t" />
            </datalist>
          </div>
        </div>

        <div class="vx-face__spacer"></div>

        <button class="vx-btn vx-face__delete" @click="remove">
          <VxIcon name="trash" :size="16" /> {{ $t('sound.delete') }}
        </button>
      </div>

      <!-- General -->
      <div v-else key="general" class="vx-face">
        <div class="vx-face__head">{{ $t('inspector.general') }}</div>

        <div class="vx-field">
          <div class="vx-field__head">
            <span><VxIcon name="volume" :size="15" /> {{ $t('inspector.masterVolume') }}</span>
          </div>
          <VxSlider :value="masterVolume" :min="0" :max="100" @input="setMaster" />
        </div>

        <div class="vx-field">
          <div class="vx-field__head">
            <span><VxIcon name="repeat" :size="15" /> {{ $t('inspector.loopEverywhere') }}</span>
          </div>
          <label class="vx-switch">
            <input type="checkbox" :checked="settings.defaultLoop" @change="toggleDefaultLoop" />
            <span class="vx-switch__track"></span>
            <span class="vx-switch__label">{{ settings.defaultLoop ? $t('common.on') : $t('common.off') }}</span>
          </label>
          <p class="vx-field__hint">{{ $t('inspector.loopEverywhereHint') }}</p>
        </div>

        <div class="vx-field">
          <div class="vx-field__head">
            <span><VxIcon name="volume" :size="15" /> {{ $t('inspector.normalize') }}</span>
          </div>
          <label class="vx-switch">
            <input type="checkbox" :checked="settings.normalize" @change="toggleNormalize" />
            <span class="vx-switch__track"></span>
            <span class="vx-switch__label">{{ settings.normalize ? $t('common.on') : $t('common.off') }}</span>
          </label>
          <p class="vx-field__hint">{{ $t('inspector.normalizeHint') }}</p>
        </div>

        <transition name="vx-collapse">
          <div v-if="settings.normalize" class="vx-field">
            <div class="vx-field__head">
              <span><VxIcon name="volume" :size="15" /> {{ $t('inspector.normalizeStrength') }}</span>
              <button v-if="settings.normalizeStrength !== 50" class="vx-link" @click="resetStrength">
                {{ $t('inspector.default') }}
              </button>
            </div>
            <VxSlider :value="settings.normalizeStrength" :min="0" :max="100" @input="setStrength" />
            <div class="vx-field__ends">
              <span>{{ $t('inspector.quieter') }}</span>
              <span>{{ $t('inspector.louder') }}</span>
            </div>
          </div>
        </transition>

        <!-- Total soundboard duration -->
        <div v-if="boardDuration != null" class="vx-board-dur">
          <VxIcon name="music" :size="13" />
          <span>{{ $t('inspector.boardDuration') }} <strong>{{ fmtMs(boardDuration) }}</strong></span>
        </div>

        <div class="vx-field">
          <button
            class="vx-btn vx-shuffle-btn"
            :class="{ 'vx-btn--accent': shuffleActive }"
            @click="toggleShuffle"
          >
            <VxIcon name="shuffle" :size="16" />
            {{ shuffleActive ? $t('inspector.shuffleStop') : $t('inspector.shuffleStart') }}
          </button>
          <p class="vx-field__hint">{{ $t('inspector.shuffleHint') }}</p>
        </div>

        <!-- Now playing (short sounds only — long sounds have the bottom playbar) -->
        <div v-if="playing && playingDurationMs < 60000" class="vx-now">
          <div class="vx-now__card">
            <div class="vx-now__art" :style="{ background: playingGradient }">
              <VxIcon name="music" :size="20" />
            </div>
            <div class="vx-now__meta">
              <span class="vx-now__name">{{ playing.sound.name }}</span>
              <div class="vx-now__bar"><span :style="{ width: progress + '%' }"></span></div>
              <div class="vx-now__times">
                <span>{{ elapsedFmt }}</span>
                <span class="vx-now__total">{{ fmtMs(playing.lengthInMs) }}</span>
                <span class="vx-now__rem">-{{ remainingFmt }}</span>
              </div>
            </div>
            <button class="vx-icon-btn" :title="$t('inspector.stop')" @click="stop">
              <VxIcon name="stop" :size="18" filled />
            </button>
          </div>
        </div>
        <div v-else-if="!playing" class="vx-now">
          <div class="vx-now__idle">
            <VxIcon name="music" :size="20" />
            <span>{{ $t('inspector.idle') }}</span>
          </div>
        </div>

        <!-- Queue panel -->
        <div class="vx-queue">
          <div class="vx-queue__head">
            <span>{{ $t('inspector.queue') }} <span v-if="queue.length" class="vx-queue__count">({{ queue.length }})</span></span>
            <span v-if="queueTotalMs > 0" class="vx-queue__total-dur">{{ fmtMs(queueTotalMs) }}</span>
            <button v-if="queue.length" class="vx-link" @click="clearQueue">{{ $t('inspector.clearQueue') }}</button>
          </div>
          <div v-if="queue.length" class="vx-queue__list">
            <div v-for="(s, i) in queue" :key="s.id + '_' + i" class="vx-queue__item">
              <div class="vx-queue__item-art" :style="{ background: tileGrad(s.name) }">
                <VxIcon name="music" :size="13" />
              </div>
              <span class="vx-queue__item-name" :title="s.name">{{ s.name }}</span>
              <span v-if="s.lengthInMs != null" class="vx-queue__item-dur">{{ fmtMs(s.lengthInMs) }}</span>
              <button class="vx-queue__item-rm" :title="$t('inspector.removeFromQueue')" @click="removeFromQueue(i)">
                <VxIcon name="close" :size="12" />
              </button>
            </div>
          </div>
          <div v-else class="vx-queue__empty">
            <span>{{ $t('inspector.queueEmpty') }}</span>
          </div>
        </div>
      </div>
    </transition>
  </aside>
</template>

<script lang="ts">
import Vue from 'vue';
import { PlayingSound, Settings, Sound } from '@/types';
import { gradientFromName, tileGradientFromName } from '@/utils/visual';
import VxIcon from '@/components/VxIcon.vue';
import VxSlider from '@/components/VxSlider.vue';

function formatMs(ms: number): string {
  if (ms < 1000) return `${(ms / 1000).toFixed(1)}s`;
  const totalSec = Math.floor(ms / 1000);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  return `${m}:${String(s).padStart(2, '0')}`;
}

export default Vue.extend({
  name: 'Inspector',
  components: { VxIcon, VxSlider },
  data: () => ({ tagDraft: '' }),
  computed: {
    suggestions(): string[] {
      const own = (this.sound?.tags || []).map(t => t.toLowerCase());
      return (this.$store.getters.tagCounts as { tag: string }[])
        .map(t => t.tag)
        .filter(t => !own.includes(t.toLowerCase()));
    },
    sound(): Sound | null {
      return this.$store.getters.selectedSound;
    },
    settings(): Settings {
      return this.$store.getters.settings;
    },
    playing(): PlayingSound | null {
      return this.$store.getters.currentSound;
    },
    queue(): Sound[] {
      return this.$store.getters.queue;
    },
    shuffleActive(): boolean {
      return this.$store.getters.shuffleActive;
    },
    queueTotalMs(): number {
      return this.queue.reduce((acc: number, s: Sound) => acc + (s.lengthInMs ?? 0), 0);
    },
    capturing(): boolean {
      return this.sound !== null && this.$store.getters.capturingHotkeyFor === this.sound.id;
    },
    keyText(): string {
      if (this.capturing) return `${this.$t('inspector.pressKeys')}`;
      return this.sound && this.sound.hotkeySequence
        ? this.sound.hotkeySequence
        : `${this.$t('sound.addKeybind')}`;
    },
    soundGradient(): string {
      return this.sound ? gradientFromName(this.sound.name) : '';
    },
    playingGradient(): string {
      return this.playing ? tileGradientFromName(this.playing.sound.name) : '';
    },
    playingDurationMs(): number {
      if (!this.playing) return 0;
      return this.playing.lengthInMs || this.playing.sound.lengthInMs || 0;
    },
    isLongSound(): boolean {
      return this.playingDurationMs >= 60000;
    },
    hasCustomVolume(): boolean {
      return !!this.sound && this.sound.localVolume !== null;
    },
    volume(): number {
      if (!this.sound) return this.settings.localVolume;
      return this.sound.localVolume ?? this.settings.localVolume;
    },
    masterVolume(): number {
      return this.settings.localVolume;
    },
    loopOverridden(): boolean {
      return !!this.sound && this.sound.repeat !== null;
    },
    effectiveLoop(): boolean {
      if (!this.sound) return false;
      return this.sound.repeat !== null ? this.sound.repeat : this.settings.defaultLoop;
    },
    progress(): number {
      if (!this.playing) return 0;
      return this.$store.getters.displayProgress * 100;
    },
    isThisPlaying(): boolean {
      return !!this.playing && !!this.sound && this.playing.sound.id === this.sound.id;
    },
    elapsedFmt(): string {
      if (!this.playing) return '0:00';
      return formatMs(this.$store.getters.displayProgress * this.playing.lengthInMs);
    },
    remainingFmt(): string {
      if (!this.playing) return '0:00';
      const rem = (1 - this.$store.getters.displayProgress) * this.playing.lengthInMs;
      return formatMs(Math.max(0, rem));
    },
    boardDuration(): number | null {
      const tab = this.$store.getters.currentTab;
      if (!tab || !tab.sounds.length) return null;
      const total = tab.sounds.reduce((acc: number, s: Sound) => acc + (s.lengthInMs ?? 0), 0);
      return total > 0 ? total : null;
    },
  },
  methods: {
    fmtMs(ms: number): string {
      return formatMs(ms);
    },
    tileGrad(name: string): string {
      return tileGradientFromName(name);
    },
    capture() {
      if (!this.sound) return;
      this.$store.commit('setCapturingHotkeyFor', this.sound.id);
      window.requestHotkey(true);
    },
    clearKey() {
      if (this.sound) this.$store.dispatch('clearHotkey', this.sound);
    },
    setVolume(v: number) {
      if (this.sound) this.$store.dispatch('setSoundVolume', { sound: this.sound, v });
    },
    resetVolume() {
      if (this.sound) this.$store.dispatch('setSoundVolume', { sound: this.sound, v: null });
    },
    toggleLoop(e: Event) {
      if (this.sound) {
        this.$store.dispatch('setSoundLoop', { sound: this.sound, loop: (e.target as HTMLInputElement).checked });
      }
    },
    resetLoop() {
      if (this.sound) this.$store.dispatch('setSoundLoop', { sound: this.sound, loop: null });
    },
    addTag() {
      const draft = this.tagDraft.trim();
      this.tagDraft = '';
      if (!this.sound || !draft) return;
      const tags = this.sound.tags || [];
      if (tags.some(t => t.toLowerCase() === draft.toLowerCase())) return;
      // Reuse an existing board tag's spelling so "meme"/"Meme" don't become two filters.
      const known = (this.$store.getters.tagCounts as { tag: string }[]).find(
        t => t.tag.toLowerCase() === draft.toLowerCase(),
      );
      this.$store.dispatch('setSoundTags', { sound: this.sound, tags: [...tags, known ? known.tag : draft] });
    },
    removeTag(tag: string) {
      if (this.sound) {
        this.$store.dispatch('setSoundTags', { sound: this.sound, tags: this.sound.tags.filter(t => t !== tag) });
      }
    },
    remove() {
      if (this.sound) this.$store.dispatch('deleteSound', this.sound);
    },
    setMaster(v: number) {
      this.$store.dispatch('setLocalVolume', v);
    },
    toggleDefaultLoop(e: Event) {
      this.$store.dispatch('setDefaultLoop', (e.target as HTMLInputElement).checked);
    },
    toggleNormalize(e: Event) {
      this.$store.dispatch('setNormalize', (e.target as HTMLInputElement).checked);
    },
    setStrength(v: number) {
      this.$store.dispatch('setNormalizeStrength', v);
    },
    resetStrength() {
      this.$store.dispatch('setNormalizeStrength', 50);
    },
    stop() {
      this.$store.dispatch('stopSounds');  // stops active tab's sound
    },
    toggleShuffle() {
      this.$store.dispatch('toggleShuffle');
    },
    clearQueue() {
      this.$store.dispatch('clearQueue');
    },
    removeFromQueue(idx: number) {
      this.$store.dispatch('removeFromQueue', idx);
    },
  },
});
</script>

<style scoped lang="scss">
.vx-inspector {
  background: var(--surface);
  border-left: 1px solid var(--border);
  overflow: hidden;
}

.vx-face {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 22px 20px;
  overflow-y: auto;

  &__cover {
    width: 100%;
    aspect-ratio: 16 / 10;
    border-radius: var(--radius);
    display: grid;
    place-items: center;
    color: oklch(1 0 0 / 0.6);
    flex: none;
  }
  &__title-row {
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex: none;
  }
  &__title {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    word-break: break-word;
    flex: 1;
    min-width: 0;
  }
  &__dur {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-dim);
    white-space: nowrap;
    flex: none;
  }
  &__head {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-dim);
  }
  &__spacer {
    flex: 1;
  }
  &__delete {
    color: var(--danger);
    border-color: transparent;
    background: oklch(0.62 0.17 25 / 0.12);
    flex: none;
    &:hover {
      background: oklch(0.62 0.17 25 / 0.2);
    }
  }
}

.vx-progress-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: none;

  &__elapsed,
  &__remaining {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-dim);
    white-space: nowrap;
    min-width: 34px;
  }
  &__remaining {
    text-align: right;
    color: var(--accent);
  }
  &__bar {
    flex: 1;
    height: 3px;
    border-radius: 99px;
    background: var(--surface-3);
    span {
      display: block;
      height: 100%;
      border-radius: 99px;
      background: var(--accent);
    }
  }
}

.vx-keybind-row {
  display: flex;
  align-items: center;
  width: 100%;
  border-radius: var(--radius-sm);
  border: 1px dashed var(--border);
  background: var(--surface-2);
  color: var(--text-dim);
  font-size: 13px;
  font-weight: 500;
  transition: all 0.14s var(--ease);
  overflow: hidden;
  flex: none;

  &__main {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    min-width: 0;
  }
  &__main span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  &__clear {
    flex: none;
    display: grid;
    place-items: center;
    width: 38px;
    align-self: stretch;
    border: none;
    border-left: 1px solid var(--border);
    background: none;
    color: var(--text-dim);
    transition: background 0.14s var(--ease), color 0.14s var(--ease);
  }
  &__clear:hover {
    background: oklch(0.62 0.17 25 / 0.16);
    color: var(--danger);
  }
  &:hover {
    color: var(--text);
    border-color: var(--text-dim);
  }
  &--set {
    color: var(--accent);
    border-style: solid;
    border-color: var(--accent);
    background: var(--accent-soft);
  }
}

.vx-field {
  display: flex;
  flex-direction: column;
  gap: 10px;
  flex: none;

  &__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
    color: var(--text);
    span {
      display: inline-flex;
      align-items: center;
      gap: 8px;
    }
  }
  &__hint {
    margin: 0;
    font-size: 12px;
    color: var(--text-dim);
  }
  &__ends {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--text-dim);
    margin-top: -2px;
  }
}

.vx-collapse-enter-active,
.vx-collapse-leave-active {
  transition: opacity 0.16s var(--ease);
}
.vx-collapse-enter,
.vx-collapse-leave-to {
  opacity: 0;
}

.vx-link {
  border: none;
  background: none;
  color: var(--accent);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
}

.vx-switch {
  display: flex;
  align-items: center;
  gap: 12px;
  cursor: pointer;

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
  &__label {
    font-size: 13px;
    color: var(--text-dim);
  }
}

.vx-board-dur {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-dim);
  flex: none;
}

.vx-shuffle-btn {
  width: 100%;
}

.vx-now {
  flex: none;
  &__card {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px;
    border-radius: var(--radius);
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  &__art {
    width: 42px;
    height: 42px;
    border-radius: var(--radius-sm);
    display: grid;
    place-items: center;
    color: oklch(1 0 0 / 0.6);
    flex: none;
  }
  &__meta {
    flex: 1;
    min-width: 0;
  }
  &__name {
    display: block;
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-bottom: 6px;
  }
  &__bar {
    height: 4px;
    border-radius: 99px;
    background: var(--surface-3);
    margin-bottom: 5px;
    span {
      display: block;
      height: 100%;
      border-radius: 99px;
      background: var(--accent);
    }
  }
  &__times {
    display: flex;
    align-items: center;
    font-size: 10px;
    color: var(--text-dim);
    gap: 4px;
  }
  &__total {
    margin-left: auto;
  }
  &__rem {
    color: var(--accent);
    font-weight: 500;
  }
  &__idle {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px;
    border-radius: var(--radius);
    border: 1px dashed var(--border);
    color: var(--text-dim);
    font-size: 13px;
  }
}

.vx-queue {
  flex: none;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;

  &__head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    background: var(--surface-2);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-dim);
    border-bottom: 1px solid var(--border);
  }
  &__count {
    font-weight: 400;
    text-transform: none;
    letter-spacing: 0;
  }
  &__total-dur {
    margin-left: auto;
    font-weight: 400;
    text-transform: none;
    letter-spacing: 0;
  }
  &__head .vx-link {
    margin-left: 4px;
    font-size: 11px;
  }

  &__list {
    max-height: 200px;
    overflow-y: auto;
  }
  &__item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    font-size: 12px;

    &:last-child {
      border-bottom: none;
    }
    &:hover {
      background: var(--surface-2);
    }
  }
  &__item-art {
    width: 26px;
    height: 26px;
    border-radius: 4px;
    display: grid;
    place-items: center;
    color: oklch(1 0 0 / 0.6);
    flex: none;
  }
  &__item-name {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text);
  }
  &__item-dur {
    color: var(--text-dim);
    font-size: 11px;
    flex: none;
  }
  &__item-rm {
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 4px;
    border: none;
    background: none;
    color: var(--text-dim);
    cursor: pointer;
    transition: background 0.12s var(--ease), color 0.12s var(--ease);

    &:hover {
      background: oklch(0.62 0.17 25 / 0.16);
      color: var(--danger);
    }
  }

  &__empty {
    padding: 12px;
    text-align: center;
    font-size: 12px;
    color: var(--text-dim);
  }
}

.vx-face-enter-active,
.vx-face-leave-active {
  transition: opacity 0.16s var(--ease), transform 0.16s var(--ease);
}
.vx-face-enter {
  opacity: 0;
  transform: translateX(10px);
}
.vx-face-leave-to {
  opacity: 0;
  transform: translateX(-10px);
}
.vx-taged {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;

  &__chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    padding: 3px 6px 3px 10px;
    border-radius: 99px;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 12px;
    font-weight: 500;

    button {
      display: grid;
      place-items: center;
      padding: 2px;
      border: none;
      border-radius: 50%;
      background: none;
      color: inherit;
      cursor: pointer;
      opacity: 0.7;

      &:hover {
        opacity: 1;
      }
    }
  }

  &__input {
    flex: 1;
    min-width: 90px;
    height: 26px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 99px;
    background: var(--surface);
    color: var(--text);
    font-size: 12px;
    outline: none;
    transition: border-color 0.14s var(--ease);

    &:focus {
      border-color: var(--accent);
    }
  }
}
</style>
