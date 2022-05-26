" Initialize the channel
if !exists('s:clipsyncJobId')
	let s:clipsyncJobId = 0
endif

if !exists('g:clipsync_bin')
  let s:bin = 'clipsync-plugin'
else
  let s:bin = g:clipsync_bin
endif

" Entry point. Initialize RPC. If it succeeds, then attach commands to the `rpcnotify` invocations.
function! s:connect()
  let id = s:initRpc()
  
  if 0 == id
    echoerr "clipsync: cannot start rpc process"
  elseif -1 == id
    echoerr "clipsync: rpc process is not executable"
  else
    " Mutate our jobId variable to hold the channel ID
    let s:clipsyncJobId = id 
    
    call s:configureCommands()
  endif
endfunction

function! s:configureCommands()
  command! -nargs=1 ClipsyncConnect :call s:clip_connect(<args>)
  command! ClipsyncDisconnect :call s:clip_disconnect(<f-args>)
endfunction

function! s:clip_connect(uri)
  echo a:uri
  call rpcnotify(s:clipsyncJobId, 'connect', a:uri)
  call s:enableAuto()
endfunction

function! s:clip_disconnect()
  call rpcnotify(s:clipsyncJobId, 'disconnect')
  call s:disableAuto()
endfunction

function! s:disableAuto()
  augroup yanks
    autocmd!
  augroup END
endfunction

function! s:enableAuto()
  augroup yanks
    autocmd!
    autocmd TextYankPost * :call s:yank(v:event['regname'])
  augroup END
endfunction

function! s:yank(regName)
  if a:regName != "+" 
    return
  endif
  let s:contents = getreg('+')
  call rpcnotify(s:clipsyncJobId, 'yank', s:contents)
endfunction

" Initialize RPC
function! s:initRpc()
  if s:clipsyncJobId == 0
    let jobid = jobstart([s:bin], { 'rpc': v:true })
    return jobid
  else
    return s:clipsyncJobId
  endif
endfunction

call s:connect()
