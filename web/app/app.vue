<template>
  <v-app>
    <v-main>
      <NuxtLayout>
        <nuxt-page />
      </NuxtLayout>
    </v-main>

    <v-btn
      aria-label="Toggle theme"
      class="ma-2"
      icon="mdi-theme-light-dark"
      location="top right"
      position="absolute"
      variant="text"
      @click="$vuetify.theme.cycle()"
    />
  </v-app>
</template>

<script setup lang="ts">
  const theme = useTheme()
  const savedTheme = useCookie('theme', {
    default: () => 'dark',
    maxAge: 60 * 60 * 24 * 365,
    sameSite: 'lax',
  })

  theme.change(savedTheme.value === 'light' ? 'light' : 'dark')

  watch(theme.name, name => {
    savedTheme.value = name
  })
</script>
