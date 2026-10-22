<template>
  <button
    class="vx-board"
    :class="{ 'vx-board--active': active }"
    :title="tab.name"
    @click="$emit('select')"
    @contextmenu.prevent="$emit('menu', $event)"
  >
    <span class="vx-board__indicator"></span>
    <span class="vx-board__face" :style="faceStyle">
      <span v-if="!coverUrl" class="vx-board__initials">{{ label }}</span>
    </span>
  </button>
</template>

<script lang="ts">
import Vue, { PropType } from 'vue';
import { Tab } from '@/types';
import { gradientFromName, initials, fileUrl } from '@/utils/visual';

export default Vue.extend({
  name: 'BoardAvatar',
  props: {
    tab: { type: Object as PropType<Tab>, required: true },
    active: { type: Boolean, default: false },
  },
  computed: {
    coverUrl(): string {
      return this.tab.cover ? fileUrl(this.tab.cover) : '';
    },
    label(): string {
      return initials(this.tab.name);
    },
    faceStyle(): Record<string, string> {
      return this.coverUrl
        ? { backgroundImage: `url("${this.coverUrl}")` }
        : { background: gradientFromName(this.tab.name) };
    },
  },
});
</script>

<style scoped lang="scss">
.vx-board {
  position: relative;
  width: 100%;
  display: grid;
  place-items: center;
  border: none;
  background: none;
  padding: 0;

  &__indicator {
    position: absolute;
    left: 0;
    top: 50%;
    transform: translateY(-50%) scaleY(0.3);
    width: 4px;
    height: 28px;
    border-radius: 0 4px 4px 0;
    background: var(--accent);
    opacity: 0;
    transition: opacity 0.18s var(--ease), transform 0.18s var(--ease);
  }

  &__face {
    width: 48px;
    height: 48px;
    border-radius: 14px;
    background-size: cover;
    background-position: center;
    display: grid;
    place-items: center;
    transition: border-radius 0.18s var(--ease), transform 0.14s var(--ease),
      box-shadow 0.18s var(--ease);
    box-shadow: 0 0 0 0 var(--accent);
  }

  &__initials {
    font-weight: 700;
    font-size: 16px;
    color: oklch(0.97 0.01 320);
    text-shadow: 0 1px 2px oklch(0 0 0 / 0.4);
  }

  &:hover &__face {
    border-radius: 16px;
    transform: scale(1.04);
  }
  &:hover &__indicator {
    opacity: 0.5;
    transform: translateY(-50%) scaleY(0.6);
  }

  &--active &__indicator {
    opacity: 1;
    transform: translateY(-50%) scaleY(1);
  }
  &--active &__face {
    border-radius: 16px;
    box-shadow: 0 0 0 2px var(--bg), 0 0 0 4px var(--accent);
  }
}
</style>
