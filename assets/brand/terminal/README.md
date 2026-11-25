# Terminal signature

The logo is `:^) nosey` in ASCII. Colour only the caret. Use the supplied light or dark variant when the host theme is known. If the theme is unknown, use the terminal's default foreground and uncoloured ASCII. Honour `NO_COLOR`, redirected output and `TERM=dumb`.

- `nosey.txt`: uncoloured fallback (9 character cells).
- `nosey-dark.ansi` and `nosey-light.ansi`: truecolour examples with SGR reset.
- `nosey.sh`: POSIX shell example with no-colour fallback; select the theme explicitly.

Do not substitute the vector wordmark for terminal text or require a specific terminal font.
