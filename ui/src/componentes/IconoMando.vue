<script setup lang="ts">
import { computed } from 'vue'
import { iconoDe } from '../mandos/iconos'

const props = withDefaults(defineProps<{ control: string; tamano?: number }>(), { tamano: 26 })
const src = computed(() => iconoDe(props.control))
</script>

<template>
  <span
    v-if="src"
    class="icono"
    role="img"
    :aria-label="control"
    :title="control"
    :style="{ maskImage: `url(&quot;${src}&quot;)`, width: `${tamano}px`, height: `${tamano}px` }"
  />
  <span v-else>{{ control }}</span>
</template>

<style scoped>
.icono {
  display: inline-block;
  flex: none;
  vertical-align: middle;
  background: currentColor;
  mask-size: contain;
  mask-repeat: no-repeat;
  mask-position: center;
}
</style>
