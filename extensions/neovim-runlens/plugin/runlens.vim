if exists('g:runlens_loaded') | finish | endif
let g:runlens_loaded = 1

lua << EOF
local runlens = require('runlens')
runlens.setup()
runlens.commands()
EOF
