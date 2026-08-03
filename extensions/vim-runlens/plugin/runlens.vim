if exists('g:runlens_loaded') | finish | endif
let g:runlens_loaded = 1

if !executable('runlens')
  echohl WarningMsg
  echom 'runlens: binary not found in $PATH - install from https://github.com/anomalyco/runlens'
  echohl None
  finish
endif

command! RunLensList   call s:list()
command! RunLensRecord  call s:record()
command! RunLensStatus  call s:status()

function! s:list()
  let out = system('runlens list --json 2>/dev/null')
  if v:shell_error
    echohl ErrorMsg | echom 'runlens: ' . out | echohl None
    return
  endif
  try
    let sessions = json_decode(out)
  catch
    echom 'runlens: no sessions'
    return
  endtry
  echom 'runlens: ' . len(sessions) . ' session(s)'
  for s in sessions
    let id = strpart(get(s, 'id', '?'), 0, 8)
    let ev = get(s, 'event_count', 0)
    let dur = get(s, 'duration_ms', 0)
    echom '  ' . id . '  ' . ev . ' events  ' . dur . 'ms'
  endfor
endfunction

function! s:record()
  let out = system('runlens record --label vim-' . localtime() . ' 2>&1')
  if v:shell_error
    echohl ErrorMsg | echom 'runlens: ' . out | echohl None
  else
    echom 'runlens: recording (' . trim(out) . ')'
  endif
endfunction

function! s:status()
  let out = system('runlens daemon status 2>/dev/null')
  if v:shell_error
    echom 'runlens: daemon not running'
  else
    echom 'runlens: ' . trim(out)
  endif
endfunction