/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
      colors: {
        // Dark Reader inspired theme: Deep navy blues
        midnight: {
          950: '#181a1b',  // Darkest - near black
          900: '#1c1e1f',  // Very dark
          800: '#232526',  // Dark panels
          700: '#2a2c2d',  // Slightly lighter
          600: '#35393a',  // Mid-dark
          500: '#454a4d',  // Mid-tone
          400: '#5c6366',  // Light mid
          300: '#8b9295',  // Light text
          200: '#b8bfc2',  // Lighter text
          100: '#e8eaec',  // Very light
        },
        // Accent colors - magenta primary
        accent: {
          magenta: '#e879f9',   // Bright magenta for primary accent
          orange: '#ff8c42',    // Orange for AI icons
          blue: '#4a9eff',      // Blue for user icons
          cyan: '#22d3ee',      // Cyan highlights
        },
        // Legacy purple/violet for gradual migration
        purple: {
          900: '#2d1b69',
          800: '#3d2784',
          700: '#4d3399',
          600: '#6644bb',
          500: '#8866dd',
        },
        violet: {
          950: '#1a1625',
          900: '#232033',
          800: '#2d2942',
          700: '#3d3654',
          600: '#4f4568',
          500: '#6b5b8a',
          400: '#8b7aad',
          300: '#a899c7',
          200: '#c4b5fd',
          100: '#ddd6fe',
        }
      },
      backgroundColor: {
        'dark': '#181a1b',
        'darker': '#121314',
      },
      animation: {
        'pulse-slow': 'pulse 3s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        'spin-slow': 'spin 3s linear infinite',
        'glow': 'glow 2s ease-in-out infinite',
        'float': 'float 3s ease-in-out infinite',
      },
      keyframes: {
        glow: {
          '0%, 100%': { textShadow: '0 0 20px rgba(74, 158, 255, 0.5)' },
          '50%': { textShadow: '0 0 30px rgba(74, 158, 255, 0.8)' },
        },
        float: {
          '0%, 100%': { transform: 'translateY(0px)' },
          '50%': { transform: 'translateY(-10px)' },
        },
      },
    },
  },
  plugins: [require('@tailwindcss/typography')],
};