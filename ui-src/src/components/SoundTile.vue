<template>
  <div
    class="vx-tile"
    :class="{ 'vx-tile--playing': isPlaying, 'vx-tile--selected': isSelected }"
    @mousedown="pressSound($event, sound, dropOnTag)"
    @click.stop="$emit(isPlaying ? 'stop' : 'play')"
    @contextmenu.prevent.stop="$emit('rightclick', $event)"
  >
    <div class="vx-tile__art" :style="{ background: gradient }">
      <VxIcon class="vx-tile__glyph" name="music" :size="26" />
      <div class="vx-tile__hover" :class="{ 'vx-tile__hover--stop': isPlaying }">
        <VxIcon :name="isPlaying ? 'stop' : 'play'" :size="26" filled />
      </div>
      <div v-if="isPlaying" class="vx-tile__bar">
        <span :style="{ width: progress + '%' }"></span>
      </div>
    </div>

    <div class="vx-tile__name" :title="sound.name">{{ sound.name }}</div>

    <button class="vx-tile__keybind" :class="{ 'vx-tile__keybind--set': hasKey }" @click.stop="$emit('keybind')">
      <VxIcon name="keyboard" :size="13" />
      <span>{{ keyLabel }}</span>
    </button>
  </div>
</template>

<script lang="ts">
import Vue, { PropType } from 'vue';
import { PlayingSound, Sound } from '@/types';
import { tileGradientFromName } from '@/utils/visual';
import VxIcon from '@/components/VxIcon.vue';
import { pressSound } from '@/utils/soundDrag';

export default Vue.extend({
  name: 'SoundTile',
  components: { VxIcon },
  props: { sound: { type: Object as PropType<Sound>, required: true } },
  methods: {
    pressSound,
    dropOnTag(sound: Sound, tag: string) {
      this.$store.dispatch('addSoundTag', { sound, tag });
    },
  },
  computed: {
    gradient(): string {
      return tileGradientFromName(this.sound.name);
    },
    current(): PlayingSound | null {
      return this.$store.getters.currentSound;
    },
    isPlaying(): boolean {
      return !!this.current && this.current.sound.id === this.sound.id;
    },
    isSelected(): boolean {
      const sel = this.$store.getters.selectedSound as Sound | null;
      return !!sel && sel.id === this.sound.id;
    },
    progress(): number {
      if (!this.isPlaying) return 0;
      return this.$store.getters.displayProgress * 100;
    },
    hasKey(): boolean {
      return !!this.sound.hotkeySequence;
    },
    keyLabel(): string {
      return this.sound.hotkeySequence || `${this.$t('sound.addKeybind')}`;
    },
  },
});
</script>

<style scoped lang="scss">
.vx-tile {
  display: flex;
  flex-direction: column;
  gap: 7px;
  min-width: 0;

  &__art {
    position: relative;
    aspect-ratio: 1 / 1;
    border-radius: var(--radius);
    overflow: hidden;
    display: grid;
    place-items: center;
    transition: transform 0.14s var(--ease), box-shadow 0.18s var(--ease);
    box-shadow: 0 0 0 0 var(--accent);
  }

  &__glyph {
    color: oklch(1 0 0 / 0.55);
    transition: opacity 0.14s var(--ease);
  }

  &__hover {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: oklch(0 0 0 / 0.34);
    color: oklch(0.99 0.01 320);
    opacity: 0;
    transition: opacity 0.14s var(--ease);
  }
  &__hover > * {
    transform: scale(0.82);
    transition: transform 0.16s var(--ease);
  }

  &:hover &__art {
    transform: translateY(-3px);
    box-shadow: 0 10px 24px oklch(0 0 0 / 0.35);
  }
  &:hover &__hover {
    opacity: 1;
  }
  &:hover &__hover > * {
    transform: scale(1);
  }
  &:hover &__glyph {
    opacity: 0;
  }

  &__bar {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 4px;
    background: oklch(0 0 0 / 0.4);
    span {
      display: block;
      height: 100%;
      background: var(--accent);
    }
  }

  &--playing &__art {
    box-shadow: 0 0 0 2px var(--bg), 0 0 0 4px var(--accent), 0 8px 22px oklch(0.7 0.19 320 / 0.35);
  }
  &--playing &__glyph {
    opacity: 0;
  }
  // show the stop affordance persistently on the playing tile
  &--playing &__hover {
    opacity: 1;
    background: oklch(0 0 0 / 0.28);
  }
  &--playing &__hover > * {
    transform: scale(1);
  }
  &__hover--stop {
    color: var(--accent);
  }
  &--selected &__name {
    color: var(--accent);
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

  &__keybind {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    margin: 0 auto;
    max-width: 100%;
    padding: 3px 9px;
    border-radius: 99px;
    border: 1px solid transparent;
    background: none;
    color: var(--text-dim);
    font-size: 11px;
    font-weight: 500;
    transition: all 0.14s var(--ease);

    span {
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    &:hover {
      color: var(--text);
      background: var(--surface-2);
    }
    &--set {
      color: var(--accent);
      border-color: var(--accent-soft);
      background: var(--accent-soft);
    }
  }
}
</style>
