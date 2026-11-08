<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import IconoMando from './IconoMando.vue'
import { estadoApp, instalarHidHide, rechazarHidHide } from '../estado/app'

/** El mismo dibujo de mando que la navegación, para que el esquema hable el idioma de la app. */
const MANDO =
  'M6 5h8c2 0 3 1.500 3.500 4l.8 4.500c.3 2-.9 3-2 3-1 0-1.600-.7-2.300-2l-.8-1.500H6.800L6 15c-.7 1.300-1.300 2-2.300 2-1.100 0-2.300-1-2-3L2.500 9C3 6.500 4 5 6 5Z M6.500 8v3 M5 9.500h3 M13.500 9.500h.01'

const dialogo = ref<HTMLDialogElement>()
const servicios = computed(() => estadoApp.value.servicios)
const visible = computed(() => servicios.value.ofrecerHidHide && servicios.value.vigemBus)
const instalando = computed(() => servicios.value.instalandoHidHide)

watch(
  visible,
  async (abrir) => {
    await nextTick()
    const el = dialogo.value
    if (!el) return
    if (abrir && !el.open) {
      el.showModal()
      el.querySelector<HTMLElement>('[data-foco-inicial]')?.focus()
    } else if (!abrir && el.open) {
      el.close()
    }
  },
  { immediate: true },
)

/** Escape, o Círculo en el mando, equivalen a "Ahora no"; mientras instala no se cierra. */
function cancelar(evento: Event) {
  evento.preventDefault()
  if (!instalando.value) rechazarHidHide()
}
</script>

<template>
  <dialog ref="dialogo" class="oferta" aria-labelledby="oferta-titulo" aria-describedby="oferta-texto" @cancel="cancelar">
    <figure class="esquema" aria-hidden="true">
      <div class="carril">
        <span class="etiqueta">Ahora</span>
        <div class="fuentes">
          <span class="fuente"><svg viewBox="0 0 20 20"><path :d="MANDO" /></svg>Tu DualSense</span>
          <span class="fuente"><svg viewBox="0 0 20 20"><path :d="MANDO" /></svg>Mando de DS5Control</span>
        </div>
        <span class="flecha" />
        <span class="resultado doble">
          <IconoMando control="R1" :tamano="28" />
          <IconoMando control="R1" :tamano="28" />
        </span>
      </div>
      <div class="carril">
        <span class="etiqueta">Con HidHide</span>
        <div class="fuentes">
          <span class="fuente oculta"><svg viewBox="0 0 20 20"><path :d="MANDO" /></svg>Tu DualSense</span>
          <span class="fuente"><svg viewBox="0 0 20 20"><path :d="MANDO" /></svg>Mando de DS5Control</span>
        </div>
        <span class="flecha" />
        <span class="resultado">
          <IconoMando control="R1" :tamano="28" />
        </span>
      </div>
    </figure>

    <div class="cuerpo">
      <h2 id="oferta-titulo">Que cada botón cuente una vez</h2>
      <p id="oferta-texto" class="entrada">
        Algunos juegos, como Fortnite, ven tu DualSense dos veces: el real y el que crea DS5Control.
        Por eso un toque de R1 salta dos pestañas. HidHide esconde el real a los juegos.
      </p>
      <ul class="detalles">
        <li>DS5Control sigue viendo tu mando; los juegos, solo el suyo.</li>
        <li>Vuelve a mostrarlo en el modo DualSense real y al cerrar la app.</li>
        <li>Gratuito, del mismo autor que ViGEmBus. Windows pedirá permiso de administrador.</li>
      </ul>
    </div>

    <footer>
      <p class="nota" aria-live="polite">
        {{ instalando ? 'Acepta el aviso de Windows para seguir.' : 'Puedes instalarlo cuando quieras desde Ajustes.' }}
      </p>
      <div class="acciones">
        <button :disabled="instalando" @click="rechazarHidHide">Ahora no</button>
        <button class="principal" data-foco-inicial :disabled="instalando" @click="instalarHidHide">
          {{ instalando ? 'Instalando...' : 'Instalar HidHide' }}
        </button>
      </div>
    </footer>
  </dialog>
</template>

<style scoped>
.oferta {
  width: min(560px, calc(100vw - 32px));
  max-height: calc(100vh - 32px);
  margin: auto;
  padding: 0;
  overflow: auto;
  border: none;
  border-radius: var(--radio-panel);
  background: var(--surface);
  color: var(--ink);
  box-shadow:
    0 0 0 1px color-mix(in srgb, var(--ink) 8%, transparent),
    0 24px 48px -16px rgba(10, 14, 22, 0.35),
    0 4px 12px -4px rgba(10, 14, 22, 0.18);
  opacity: 1;
  transform: scale(1);
  transition:
    opacity 220ms var(--salida-rapida),
    transform 220ms var(--salida-rapida);
}

@starting-style {
  .oferta[open] {
    opacity: 0;
    transform: scale(0.96);
  }
}

.oferta::backdrop {
  background: rgba(10, 14, 22, 0.5);
}

/* Esquema: el problema dibujado antes que explicado. */
.esquema {
  display: grid;
  gap: 10px;
  margin: 0;
  padding: 20px 24px;
  background: var(--ground);
}

.carril {
  display: grid;
  grid-template-columns: 92px max-content minmax(32px, 1fr) 64px;
  align-items: center;
  gap: 14px;
}

.etiqueta {
  color: var(--muted);
  font-size: 13px;
  font-weight: 600;
}

.fuentes {
  display: grid;
  gap: 4px;
}

.fuente {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 500;
}

.fuente svg {
  flex: none;
  width: 18px;
  height: 18px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.5;
  stroke-linecap: round;
  stroke-linejoin: round;
}

.fuente.oculta {
  color: var(--guia);
  text-decoration: line-through;
  text-decoration-thickness: 1px;
}

.flecha {
  position: relative;
  height: 1px;
  background: var(--guia);
}

.flecha::after {
  content: '';
  position: absolute;
  right: -1px;
  top: -3px;
  width: 7px;
  height: 7px;
  border-top: 1px solid var(--guia);
  border-right: 1px solid var(--guia);
  transform: rotate(45deg);
}

.resultado {
  display: flex;
  gap: 4px;
  color: var(--accent);
}

.resultado.doble {
  color: var(--aviso);
}

.cuerpo {
  display: grid;
  gap: 10px;
  padding: 24px 24px 4px;
}

h2 {
  font-size: 22px;
  font-weight: 700;
  line-height: 1.2;
  letter-spacing: -0.01em;
  text-wrap: balance;
}

.entrada {
  font-size: 15px;
  line-height: 1.5;
  text-wrap: pretty;
}

.detalles {
  display: grid;
  gap: 6px;
  margin: 4px 0 0;
  padding: 0;
  list-style: none;
}

.detalles li {
  position: relative;
  padding-left: 16px;
  color: var(--muted);
  font-size: 14px;
  line-height: 1.45;
  text-wrap: pretty;
}

.detalles li::before {
  content: '';
  position: absolute;
  left: 2px;
  top: 0.62em;
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--guia);
}

footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 12px 16px;
  padding: 20px 24px 24px;
}

.nota {
  color: var(--muted);
  font-size: 13px;
}

.acciones {
  display: flex;
  gap: 8px;
  margin-left: auto;
}

button {
  min-height: 44px;
  padding: 0 18px;
  border-radius: var(--radio-control);
  background: var(--ground);
  font-weight: 600;
  transition:
    transform 140ms var(--salida-rapida),
    background-color 140ms var(--salida-rapida);
}

@media (hover: hover) and (pointer: fine) {
  button:hover:not(:disabled) {
    background: var(--linea);
  }

  .principal:hover:not(:disabled) {
    background: var(--ink);
  }
}

button:active:not(:disabled) {
  transform: scale(0.97);
}

.principal {
  background: var(--ink);
  color: var(--surface);
}

button:disabled {
  opacity: 0.6;
}

@media (max-width: 520px) {
  .carril {
    grid-template-columns: max-content minmax(20px, 1fr) 64px;
  }

  .etiqueta {
    grid-column: 1 / -1;
  }
}

@media (prefers-reduced-motion: reduce) {
  .oferta {
    transition: opacity 160ms ease;
    transform: none;
  }

  @starting-style {
    .oferta[open] {
      transform: none;
    }
  }
}
</style>
