/** @type {import('tailwindcss').Config} */
export default {
    content: [
        "./index.html",
        "./src/**/*.{vue,js,ts,jsx,tsx}",
    ],
    theme: {
        extend: {
            colors: {
                // Sci-Fi Palette (Mapped to CSS Variables)
                'sci-base': 'var(--bg-app)',
                'sci-panel': 'var(--bg-panel)',
                'sci-panel-hover': 'var(--bg-panel-hover)',
                'sci-border': 'var(--border-color)',
                'sci-border-active': 'var(--border-active)',

                'sci-text-pri': 'var(--text-main)',
                'sci-text-sec': 'var(--text-muted)',
                'sci-text-dim': 'var(--text-dim)',

                'sci-cyan': 'var(--color-primary)',
                'sci-cyan-dim': 'var(--color-primary-dim)',
                'sci-orange': 'var(--color-secondary)',
                'sci-orange-dim': 'var(--color-secondary-dim)',
                'sci-success': 'var(--color-success)',
                'sci-success-dim': 'var(--color-success-dim)',
                'sci-danger': 'var(--color-danger)',
                'sci-danger-dim': 'var(--color-danger-dim)',
            },
            fontFamily: {
                sans: ['var(--font-ui)', 'sans-serif'],
                mono: ['var(--font-mono)', 'monospace'],
                display: ['Rajdhani', 'sans-serif'], // Keep Rajdhani as display font if available, or fallback
            }
        },
    },
    plugins: [
        require('@tailwindcss/typography'),
    ],
}
